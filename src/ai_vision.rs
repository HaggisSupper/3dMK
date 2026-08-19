use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use std::path::Path;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct FloorplanWallSegment {
    pub start: [f64; 2],
    pub end: [f64; 2],
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub kind: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct FloorplanCorner {
    pub point: [f64; 2],
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub angle_degrees: f64,
    #[serde(default)]
    pub kind: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct FloorplanRoom {
    #[serde(default)]
    pub name: String,
    pub boundary: Vec<[f64; 2]>,
    #[serde(default)]
    pub raw_boundary: Vec<[f64; 2]>,
    #[serde(default)]
    pub wall_segments: Vec<FloorplanWallSegment>,
    #[serde(default)]
    pub corners: Vec<FloorplanCorner>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct FloorplanLayout {
    pub outer_boundary: Vec<[f64; 2]>,
    #[serde(default)]
    pub raw_outer_boundary: Vec<[f64; 2]>,
    #[serde(default)]
    pub outer_walls: Vec<FloorplanWallSegment>,
    #[serde(default)]
    pub outer_corners: Vec<FloorplanCorner>,
    #[serde(default)]
    pub rooms: Vec<FloorplanRoom>,
    #[serde(default)]
    pub source: String,
}

fn configured_vlm_endpoint() -> String {
    std::env::var("THREEDMK_VLM_ENDPOINT").unwrap_or_else(|_| mistral_endpoint())
}

fn configured_vlm_model() -> String {
    std::env::var("THREEDMK_VLM_MODEL").unwrap_or_else(|_| "pixtral".to_string())
}

fn configured_vlm_api_key() -> String {
    std::env::var("THREEDMK_VLM_API_KEY").unwrap_or_default()
}

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct VlmObjectDetectionOptions {
    #[serde(skip_deserializing, default = "configured_vlm_endpoint")]
    pub endpoint: String,
    #[serde(skip_deserializing, default = "configured_vlm_model")]
    pub model: String,
    #[serde(skip_deserializing, default = "configured_vlm_api_key")]
    pub api_key: String,
    pub candidate_labels: Vec<String>,
    pub confidence_threshold: f64,
}

impl Default for VlmObjectDetectionOptions {
    fn default() -> Self {
        Self {
            endpoint: configured_vlm_endpoint(),
            model: configured_vlm_model(),
            api_key: configured_vlm_api_key(),
            candidate_labels: vec![
                "door".to_string(),
                "window".to_string(),
                "chair".to_string(),
                "table".to_string(),
                "sofa".to_string(),
                "bed".to_string(),
                "cabinet".to_string(),
                "appliance".to_string(),
            ],
            confidence_threshold: 0.35,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct VlmObjectDetection {
    pub label: String,
    pub confidence: f64,
    /// Normalized [left, top, right, bottom] coordinates.
    pub bbox: [f64; 4],
}

#[derive(Clone, Debug, Serialize)]
pub struct VlmObjectDetectionBatch {
    pub source_id: String,
    pub model: String,
    pub image_width: u32,
    pub image_height: u32,
    pub detections: Vec<VlmObjectDetection>,
}

#[derive(Deserialize)]
struct RawVlmDetectionEnvelope {
    #[serde(default)]
    detections: Vec<RawVlmDetection>,
}

#[derive(Deserialize)]
struct RawVlmDetection {
    label: String,
    #[serde(default = "default_detection_confidence")]
    confidence: f64,
    #[serde(alias = "box_2d", alias = "bounding_box")]
    bbox: [f64; 4],
}

fn default_detection_confidence() -> f64 {
    0.5
}

fn validate_local_ai_image_budget(bytes: &[u8]) -> Result<(u32, u32)> {
    if bytes.len() > crate::perception::MAX_VWM_PERCEPTION_IMAGE_BYTES {
        anyhow::bail!("Local AI image exceeds the compressed-byte budget");
    }
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .context("Local AI could not determine the supplied image format")?;
    let (width, height) = reader
        .into_dimensions()
        .context("Local AI could not read the supplied image dimensions")?;
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .context("Local AI image dimensions overflow")?;
    if width == 0 || height == 0 || pixels > crate::perception::MAX_VWM_PERCEPTION_PIXELS {
        anyhow::bail!("Local AI image exceeds the decoded-pixel budget");
    }
    Ok((width, height))
}

fn local_ai_http_client(timeout: std::time::Duration) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .build()
        .context("Could not create the local AI HTTP client")
}

fn local_vlm_completion_url(endpoint: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(endpoint.trim().trim_end_matches('/'))
        .context("VLM endpoint is not a valid URL")?;
    if url.scheme() != "http" {
        anyhow::bail!("VLM endpoint must use loopback HTTP");
    }
    let host = url.host_str().unwrap_or_default();
    if !matches!(host, "127.0.0.1" | "localhost" | "::1" | "[::1]") {
        anyhow::bail!("VLM endpoint must use a loopback host");
    }
    if !url.username().is_empty() || url.password().is_some() {
        anyhow::bail!("VLM endpoint must not contain URL credentials");
    }
    if url.query().is_some() || url.fragment().is_some() {
        anyhow::bail!("VLM endpoint must not contain a query or fragment");
    }
    let normalized_path = url.path().trim_end_matches('/').to_owned();
    match normalized_path.as_str() {
        "" | "/" => url.set_path("/v1/chat/completions"),
        "/v1/chat/completions" | "/chat/completions" => {}
        _ => anyhow::bail!(
            "VLM endpoint must be a loopback server origin or chat-completions endpoint"
        ),
    }
    Ok(url.to_string())
}

pub async fn detect_objects_vlm(
    source_id: &str,
    image_bytes: &[u8],
    mut options: VlmObjectDetectionOptions,
) -> Result<VlmObjectDetectionBatch> {
    options.endpoint = configured_vlm_endpoint()
        .trim()
        .trim_end_matches('/')
        .to_string();
    options.model = configured_vlm_model().trim().to_string();
    options.api_key = configured_vlm_api_key();
    let completion_url = local_vlm_completion_url(&options.endpoint)?;
    if options.model.is_empty() {
        anyhow::bail!("VLM model name is required");
    }
    if !options.confidence_threshold.is_finite()
        || !(0.0..=1.0).contains(&options.confidence_threshold)
    {
        anyhow::bail!("VLM confidence threshold must be between 0 and 1");
    }
    let (image_width, image_height) = validate_local_ai_image_budget(image_bytes)?;
    let decoded = image::load_from_memory(image_bytes)
        .context("VLM object detection could not decode the supplied image")?;
    if decoded.width() != image_width || decoded.height() != image_height {
        anyhow::bail!("Local AI image dimensions changed during decode");
    }
    let mime = match Path::new(source_id)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("jpg" | "jpeg") => "image/jpeg",
        _ => "image/png",
    };
    let labels = options
        .candidate_labels
        .iter()
        .map(|label| label.trim())
        .filter(|label| !label.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    let prompt = format!(
        "Detect every visible object{} and return ONLY JSON using this exact schema: {{\"detections\":[{{\"label\":\"chair\",\"confidence\":0.93,\"bbox\":[left,top,right,bottom]}}]}}. Bounding coordinates must be integers from 0 to 1000 relative to the image. Do not add prose, masks, markdown, or invented objects.",
        if labels.is_empty() {
            String::new()
        } else {
            format!(" from these preferred classes: {labels}")
        }
    );
    let payload = serde_json::json!({
        "model": options.model,
        "messages": [{
            "role": "user",
            "content": [
                { "type": "image_url", "image_url": { "url": format!("data:{};base64,{}", mime, base64_encode(image_bytes)) } },
                { "type": "text", "text": prompt }
            ]
        }],
        "temperature": 0.0,
        "max_tokens": 1800
    });

    let client = local_ai_http_client(std::time::Duration::from_secs(120))?;
    let mut request = client.post(&completion_url).json(&payload);
    if !options.api_key.trim().is_empty() {
        request = request.bearer_auth(options.api_key.trim());
    }
    let response = request
        .send()
        .await
        .with_context(|| format!("Failed to connect to VLM service at {}", options.endpoint))?;
    if !response.status().is_success() {
        anyhow::bail!("VLM service returned {}", response.status());
    }
    let content = response
        .json::<ChatResponse>()
        .await
        .context("Failed to parse VLM service response")?
        .choices
        .into_iter()
        .next()
        .context("VLM service returned no choices")?
        .message
        .content;
    let json = extract_json(&content).context("VLM did not return a JSON detection object")?;
    let raw: RawVlmDetectionEnvelope =
        serde_json::from_str(json).context("VLM returned invalid detection JSON")?;
    let mut detections = raw
        .detections
        .into_iter()
        .filter_map(|item| {
            let divisor = if item.bbox.iter().copied().fold(0.0_f64, f64::max) <= 1.0 {
                1.0
            } else {
                1000.0
            };
            let bbox = [
                (item.bbox[0] / divisor).clamp(0.0, 1.0),
                (item.bbox[1] / divisor).clamp(0.0, 1.0),
                (item.bbox[2] / divisor).clamp(0.0, 1.0),
                (item.bbox[3] / divisor).clamp(0.0, 1.0),
            ];
            let confidence = item.confidence.clamp(0.0, 1.0);
            (!item.label.trim().is_empty()
                && confidence >= options.confidence_threshold
                && bbox[2] > bbox[0]
                && bbox[3] > bbox[1])
                .then(|| VlmObjectDetection {
                    label: item.label.trim().to_string(),
                    confidence,
                    bbox,
                })
        })
        .collect::<Vec<_>>();
    detections.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    detections.truncate(100);
    Ok(VlmObjectDetectionBatch {
        source_id: source_id.to_string(),
        model: options.model,
        image_width,
        image_height,
        detections,
    })
}

impl FloorplanLayout {
    pub fn enrich_features(&mut self) {
        let raw_outer = if self.raw_outer_boundary.len() >= 3 {
            normalize_boundary(&self.raw_outer_boundary)
        } else {
            normalize_boundary(&self.outer_boundary)
        };
        let regularized_outer = if self.outer_boundary.len() >= 3 {
            regularize_boundary(&self.outer_boundary)
        } else {
            regularize_boundary(&raw_outer)
        };
        self.raw_outer_boundary = raw_outer;
        self.outer_boundary = regularized_outer;
        self.outer_walls = build_wall_segments(&self.outer_boundary, "exterior_wall");
        self.outer_corners = build_corners(&self.outer_boundary, "exterior_corner");

        for room in &mut self.rooms {
            let raw_boundary = if room.raw_boundary.len() >= 3 {
                normalize_boundary(&room.raw_boundary)
            } else {
                normalize_boundary(&room.boundary)
            };
            let regularized_boundary = if room.boundary.len() >= 3 {
                regularize_boundary(&room.boundary)
            } else {
                regularize_boundary(&raw_boundary)
            };
            room.raw_boundary = raw_boundary;
            room.boundary = regularized_boundary;
            room.wall_segments = build_wall_segments(&room.boundary, "room_wall");
            room.corners = build_corners(&room.boundary, "room_corner");
        }
    }
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

pub async fn vision_available() -> bool {
    vision_status().await.reachable
}

#[derive(Clone, Debug, Serialize)]
pub struct VisionRuntimeStatus {
    pub endpoint: String,
    pub model: String,
    pub reachable: bool,
    pub message: String,
}

pub async fn vision_status() -> VisionRuntimeStatus {
    let model = configured_vlm_model().trim().to_owned();
    let endpoint = match local_vlm_completion_url(&configured_vlm_endpoint()) {
        Ok(completion_url) => {
            let mut url = match reqwest::Url::parse(&completion_url) {
                Ok(url) => url,
                Err(_) => {
                    return VisionRuntimeStatus {
                        endpoint: "loopback VLM endpoint".to_owned(),
                        model,
                        reachable: false,
                        message: "The configured VLM endpoint is invalid.".to_owned(),
                    }
                }
            };
            url.set_path("");
            url.set_query(None);
            url.set_fragment(None);
            url.to_string().trim_end_matches('/').to_owned()
        }
        Err(_) => {
            return VisionRuntimeStatus {
                endpoint: "loopback VLM endpoint".to_owned(),
                model,
                reachable: false,
                message: "Configure a loopback HTTP VLM endpoint before running perception."
                    .to_owned(),
            }
        }
    };
    let client = match local_ai_http_client(std::time::Duration::from_secs(2)) {
        Ok(client) => client,
        Err(error) => {
            return VisionRuntimeStatus {
                endpoint,
                model,
                reachable: false,
                message: format!("Could not create the local VLM health client: {error}"),
            }
        }
    };
    match client.get(format!("{endpoint}/v1/models")).send().await {
        Ok(response) if response.status().is_success() => VisionRuntimeStatus {
            endpoint,
            model,
            reachable: true,
            message: "Local VLM responded on its OpenAI-compatible models endpoint.".to_owned(),
        },
        Ok(response) => VisionRuntimeStatus {
            endpoint,
            model,
            reachable: false,
            message: format!(
                "Local VLM returned HTTP {} from /v1/models.",
                response.status()
            ),
        },
        Err(error) => VisionRuntimeStatus {
            endpoint,
            model,
            reachable: false,
            message: format!("Local VLM is not reachable: {error}"),
        },
    }
}

/// Extract a room-aware layout with local vision when configured, otherwise a deterministic raster outline.
pub async fn extract_floorplan_layout(image_path: &str) -> Result<FloorplanLayout> {
    let path = Path::new(image_path);
    if !path.is_file() {
        anyhow::bail!("Image file not found: {}", image_path);
    }
    if !vision_available().await {
        return extract_floorplan_outline(path);
    }
    match extract_floorplan_mistral_layout(path).await {
        Ok(layout) => Ok(layout),
        Err(error) => {
            eprintln!(
                "Vision layout extraction unavailable ({error}); using local outline fallback."
            );
            extract_floorplan_outline(path)
        }
    }
}

/// Backwards-compatible CLI helper.
pub async fn extract_floorplan_mistral(image_path: &str) -> Result<Vec<(f64, f64)>> {
    Ok(extract_floorplan_layout(image_path)
        .await?
        .outer_boundary
        .into_iter()
        .map(|point| (point[0], point[1]))
        .collect())
}

async fn extract_floorplan_mistral_layout(path: &Path) -> Result<FloorplanLayout> {
    let image_bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read image file: {}", path.display()))?;
    validate_local_ai_image_budget(&image_bytes)?;
    let mime = match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("jpg" | "jpeg") => "image/jpeg",
        _ => "image/png",
    };
    let prompt = "You are a floorplan layout engine. Return ONLY JSON with this exact schema: {\"outer_boundary\":[[x,y],...],\"rooms\":[{\"name\":\"Room\",\"boundary\":[[x,y],...]}]}. Coordinates are metres. Trace the exterior walls and each enclosed room. Do not invent scale or add prose.";
    let payload = serde_json::json!({
        "model": "pixtral",
        "messages": [{
            "role": "user",
            "content": [
                { "type": "image_url", "image_url": { "url": format!("data:{};base64,{}", mime, base64_encode(&image_bytes)) } },
                { "type": "text", "text": prompt }
            ]
        }],
        "temperature": 0.0,
        "max_tokens": 1200
    });
    let endpoint = mistral_endpoint();
    let response = local_ai_http_client(std::time::Duration::from_secs(120))?
        .post(format!("{}/v1/chat/completions", endpoint))
        .json(&payload)
        .send()
        .await
        .with_context(|| format!("Failed to connect to local vision service at {}", endpoint))?;
    if !response.status().is_success() {
        anyhow::bail!("Vision service returned {}", response.status());
    }
    let content = response
        .json::<ChatResponse>()
        .await
        .context("Failed to parse vision service response")?
        .choices
        .into_iter()
        .next()
        .context("Vision service returned no choices")?
        .message
        .content;
    let json = extract_json(&content).context("Vision service did not return JSON layout")?;
    let mut layout = if json.trim_start().starts_with('[') {
        FloorplanLayout {
            outer_boundary: serde_json::from_str(json)
                .context("Invalid legacy wall boundary response")?,
            raw_outer_boundary: Vec::new(),
            outer_walls: Vec::new(),
            outer_corners: Vec::new(),
            rooms: Vec::new(),
            source: String::new(),
        }
    } else {
        serde_json::from_str(json).context("Invalid floorplan layout response")?
    };
    layout.source = "mistral_vision".to_string();
    layout.enrich_features();
    validate_layout(&layout)?;
    Ok(layout)
}

fn extract_floorplan_outline(path: &Path) -> Result<FloorplanLayout> {
    let image = image::open(path)
        .with_context(|| format!("Could not decode floorplan image: {}", path.display()))?;
    let grayscale = image.to_luma8();
    let (width, height) = grayscale.dimensions();
    if width < 8 || height < 8 {
        anyhow::bail!("Floorplan image is too small to analyse");
    }
    let margin = (width.min(height) / 100).clamp(1, 12);
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut dark_pixels = 0usize;
    for y in margin..height - margin {
        for x in margin..width - margin {
            if grayscale.get_pixel(x, y)[0] < 170 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                dark_pixels += 1;
            }
        }
    }
    if dark_pixels == 0 || max_x <= min_x || max_y <= min_y {
        anyhow::bail!("No floorplan wall marks were found in the image");
    }
    let plan_width = (max_x - min_x) as f64;
    let plan_height = (max_y - min_y) as f64;
    let scale = 10.0 / plan_width.max(1.0); // ponytail: no drawing scale is available; normalise the local fallback to 10 m wide.
    let layout = FloorplanLayout {
        outer_boundary: vec![
            [0.0, 0.0],
            [plan_width * scale, 0.0],
            [plan_width * scale, plan_height * scale],
            [0.0, plan_height * scale],
        ],
        raw_outer_boundary: Vec::new(),
        outer_walls: Vec::new(),
        outer_corners: Vec::new(),
        rooms: Vec::new(),
        source: "local_outline".to_string(),
    };
    let mut layout = layout;
    layout.enrich_features();
    validate_layout(&layout)?;
    Ok(layout)
}

fn validate_layout(layout: &FloorplanLayout) -> Result<()> {
    if layout.outer_boundary.len() < 3 {
        anyhow::bail!("Floorplan needs at least three exterior boundary points");
    }
    for polygon in std::iter::once(&layout.outer_boundary)
        .chain(layout.rooms.iter().map(|room| &room.boundary))
    {
        if polygon.len() < 3 || polygon.iter().flatten().any(|value| !value.is_finite()) {
            anyhow::bail!("Floorplan contains an invalid boundary");
        }
    }
    Ok(())
}

fn normalize_boundary(boundary: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut normalized = boundary
        .iter()
        .copied()
        .filter(|point| point[0].is_finite() && point[1].is_finite())
        .collect::<Vec<_>>();
    if normalized.len() >= 2 && distance_2d(normalized[0], *normalized.last().unwrap()) <= 1e-6 {
        normalized.pop();
    }
    normalized.dedup_by(|left, right| distance_2d(*left, *right) <= 1e-6);
    normalized
}

fn regularize_boundary(boundary: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut simplified = merge_collinear_points(normalize_boundary(boundary));
    if simplified.len() < 3 {
        return simplified;
    }

    let Some(dominant_angle) = dominant_axis_angle(&simplified) else {
        return simplified;
    };
    // ponytail: regularize every closed footprint to its measured major/minor axes;
    // preserve the raw boundary alongside it when a non-Manhattan shape needs review.
    let rotated = simplified
        .iter()
        .map(|point| rotate_2d(*point, -dominant_angle))
        .collect::<Vec<_>>();
    let mut edges = Vec::with_capacity(rotated.len());
    for index in 0..rotated.len() {
        let start = rotated[index];
        let end = rotated[(index + 1) % rotated.len()];
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        if dx.abs() >= dy.abs() {
            edges.push(RegularizedEdge::Horizontal((start[1] + end[1]) * 0.5));
        } else {
            edges.push(RegularizedEdge::Vertical((start[0] + end[0]) * 0.5));
        }
    }

    let mut snapped = Vec::with_capacity(rotated.len());
    for index in 0..rotated.len() {
        let current = edges[index];
        let previous = edges[(index + edges.len() - 1) % edges.len()];
        let fallback = rotated[index];
        let point = match (previous, current) {
            (RegularizedEdge::Horizontal(y), RegularizedEdge::Vertical(x))
            | (RegularizedEdge::Vertical(x), RegularizedEdge::Horizontal(y)) => [x, y],
            _ => fallback,
        };
        snapped.push(rotate_2d(point, dominant_angle));
    }

    simplified = merge_collinear_points(snapped);
    if simplified.len() < 3 {
        normalize_boundary(boundary)
    } else {
        simplified
    }
}

fn build_wall_segments(boundary: &[[f64; 2]], kind: &str) -> Vec<FloorplanWallSegment> {
    let points = normalize_boundary(boundary);
    if points.len() < 2 {
        return Vec::new();
    }
    let lengths = (0..points.len())
        .map(|index| distance_2d(points[index], points[(index + 1) % points.len()]))
        .collect::<Vec<_>>();
    let max_length = lengths.iter().copied().fold(1e-6, f64::max);
    (0..points.len())
        .filter_map(|index| {
            let start = points[index];
            let end = points[(index + 1) % points.len()];
            let length = lengths[index];
            (length > 1e-6).then(|| FloorplanWallSegment {
                start,
                end,
                confidence: (0.35 + 0.65 * (length / max_length)).clamp(0.0, 1.0),
                kind: kind.to_string(),
            })
        })
        .collect()
}

fn build_corners(boundary: &[[f64; 2]], kind: &str) -> Vec<FloorplanCorner> {
    let points = normalize_boundary(boundary);
    if points.len() < 3 {
        return Vec::new();
    }
    let lengths = (0..points.len())
        .map(|index| distance_2d(points[index], points[(index + 1) % points.len()]))
        .collect::<Vec<_>>();
    let max_length = lengths.iter().copied().fold(1e-6, f64::max);

    (0..points.len())
        .map(|index| {
            let previous = points[(index + points.len() - 1) % points.len()];
            let current = points[index];
            let next = points[(index + 1) % points.len()];
            let angle = corner_angle_degrees(previous, current, next);
            let turn_strength = ((180.0 - angle).abs() / 180.0).clamp(0.0, 1.0);
            let length_factor = (lengths[(index + points.len() - 1) % points.len()]
                .min(lengths[index])
                / max_length)
                .clamp(0.0, 1.0);
            FloorplanCorner {
                point: current,
                confidence: (0.25 + turn_strength * 0.45 + length_factor * 0.3).clamp(0.0, 1.0),
                angle_degrees: angle,
                kind: kind.to_string(),
            }
        })
        .collect()
}

fn merge_collinear_points(mut boundary: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    boundary = normalize_boundary(&boundary);
    if boundary.len() < 3 {
        return boundary;
    }
    let span = boundary.iter().fold(
        ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]),
        |acc, point| {
            (
                [acc.0[0].min(point[0]), acc.0[1].min(point[1])],
                [acc.1[0].max(point[0]), acc.1[1].max(point[1])],
            )
        },
    );
    let diagonal = distance_2d(span.0, span.1).max(1.0);
    let tolerance = diagonal * 0.015;

    let mut changed = true;
    while changed && boundary.len() >= 3 {
        changed = false;
        let mut next = Vec::with_capacity(boundary.len());
        for index in 0..boundary.len() {
            let previous = boundary[(index + boundary.len() - 1) % boundary.len()];
            let current = boundary[index];
            let following = boundary[(index + 1) % boundary.len()];
            if point_line_distance_2d(current, previous, following) <= tolerance {
                changed = true;
                continue;
            }
            next.push(current);
        }
        if next.len() < 3 {
            break;
        }
        boundary = normalize_boundary(&next);
    }
    boundary
}

fn point_line_distance_2d(point: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let ap = [point[0] - a[0], point[1] - a[1]];
    let length_sq = ab[0] * ab[0] + ab[1] * ab[1];
    if length_sq <= 1e-12 {
        return distance_2d(point, a);
    }
    let t = ((ap[0] * ab[0] + ap[1] * ab[1]) / length_sq).clamp(0.0, 1.0);
    let projection = [a[0] + ab[0] * t, a[1] + ab[1] * t];
    distance_2d(point, projection)
}

fn dominant_axis_angle(boundary: &[[f64; 2]]) -> Option<f64> {
    let mut weighted_cos = 0.0;
    let mut weighted_sin = 0.0;
    let mut total_weight = 0.0;
    for index in 0..boundary.len() {
        let start = boundary[index];
        let end = boundary[(index + 1) % boundary.len()];
        let length = distance_2d(start, end);
        if length <= 1e-6 {
            continue;
        }
        let angle = (end[1] - start[1]).atan2(end[0] - start[0]);
        weighted_cos += length * (2.0 * angle).cos();
        weighted_sin += length * (2.0 * angle).sin();
        total_weight += length;
    }
    (total_weight > 0.0).then(|| normalize_angle(0.5 * weighted_sin.atan2(weighted_cos)))
}

fn rotate_2d(point: [f64; 2], angle: f64) -> [f64; 2] {
    let (sin, cos) = angle.sin_cos();
    [
        point[0] * cos - point[1] * sin,
        point[0] * sin + point[1] * cos,
    ]
}

fn normalize_angle(angle: f64) -> f64 {
    angle.rem_euclid(PI)
}

fn corner_angle_degrees(previous: [f64; 2], current: [f64; 2], next: [f64; 2]) -> f64 {
    let left = [previous[0] - current[0], previous[1] - current[1]];
    let right = [next[0] - current[0], next[1] - current[1]];
    let left_length = (left[0] * left[0] + left[1] * left[1]).sqrt();
    let right_length = (right[0] * right[0] + right[1] * right[1]).sqrt();
    if left_length <= 1e-9 || right_length <= 1e-9 {
        return 180.0;
    }
    let cosine = ((left[0] * right[0]) + (left[1] * right[1])) / (left_length * right_length);
    cosine.clamp(-1.0, 1.0).acos().to_degrees()
}

fn distance_2d(left: [f64; 2], right: [f64; 2]) -> f64 {
    ((left[0] - right[0]).powi(2) + (left[1] - right[1]).powi(2)).sqrt()
}

#[derive(Clone, Copy)]
enum RegularizedEdge {
    Horizontal(f64),
    Vertical(f64),
}

fn extract_json(content: &str) -> Option<&str> {
    let start = content.find(['{', '['])?;
    let end = content.rfind(['}', ']'])?;
    (end >= start).then(|| &content[start..=end])
}

fn local_mistral_origin(endpoint: &str) -> Option<String> {
    let mut url = reqwest::Url::parse(endpoint.trim().trim_end_matches('/')).ok()?;
    if url.scheme() != "http" {
        return None;
    }
    if !matches!(
        url.host_str().unwrap_or_default(),
        "127.0.0.1" | "localhost" | "::1" | "[::1]"
    ) {
        return None;
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
    {
        return None;
    }
    url.set_path("");
    Some(url.to_string().trim_end_matches('/').to_string())
}

fn mistral_endpoint() -> String {
    std::env::var("MISTRAL_ENDPOINT")
        .ok()
        .and_then(|endpoint| local_mistral_origin(&endpoint))
        .unwrap_or_else(|| "http://127.0.0.1:8080".to_string())
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let triple = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        result.push(if chunk.len() > 1 {
            CHARS[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        result.push(if chunk.len() > 2 {
            CHARS[(triple & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ai_image_budget_rejects_oversized_payloads() {
        let oversized = vec![0_u8; crate::perception::MAX_VWM_PERCEPTION_IMAGE_BYTES + 1];
        let error = validate_local_ai_image_budget(&oversized)
            .unwrap_err()
            .to_string();
        assert!(error.contains("compressed-byte budget"));
    }

    #[tokio::test]
    async fn local_ai_http_client_does_not_follow_redirects() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 4096];
            stream.read(&mut request).await.unwrap();
            stream
                .write_all(
                    b"HTTP/1.1 302 Found\r\nLocation: http://example.com/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await
                .unwrap();
            stream.shutdown().await.unwrap();
        });

        let response = local_ai_http_client(std::time::Duration::from_secs(5))
            .unwrap()
            .get(format!("http://{address}"))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_redirection());
        server.await.unwrap();
    }

    #[test]
    fn mistral_endpoint_origin_is_loopback_only() {
        for endpoint in [
            "http://127.0.0.1:8080",
            "http://localhost:8080",
            "http://[::1]:8080/",
        ] {
            assert!(local_mistral_origin(endpoint).is_some(), "{endpoint}");
        }
        for endpoint in [
            "https://127.0.0.1:8080",
            "http://example.com:8080",
            "http://user:password@127.0.0.1:8080",
            "http://127.0.0.1:8080/proxy",
            "http://127.0.0.1:8080?target=remote",
            "file:///tmp/model",
        ] {
            assert!(local_mistral_origin(endpoint).is_none(), "{endpoint}");
        }
    }

    #[test]
    fn vlm_endpoint_is_loopback_only() {
        for endpoint in [
            "http://127.0.0.1:8080",
            "http://localhost:8080/v1/chat/completions",
            "http://[::1]:8080/chat/completions",
        ] {
            assert!(local_vlm_completion_url(endpoint).is_ok(), "{endpoint}");
        }
        for endpoint in [
            "https://127.0.0.1:8080",
            "http://example.com:8080",
            "http://user:password@127.0.0.1:8080",
            "http://127.0.0.1:8080/proxy",
            "file:///tmp/model",
        ] {
            assert!(local_vlm_completion_url(endpoint).is_err(), "{endpoint}");
        }
    }

    #[test]
    fn request_payload_cannot_override_vlm_connection_or_credentials() {
        let options: VlmObjectDetectionOptions = serde_json::from_value(serde_json::json!({
            "endpoint": "http://example.com:9000",
            "model": "remote-model",
            "api_key": "request-secret",
            "candidate_labels": ["door"],
            "confidence_threshold": 0.4
        }))
        .unwrap();
        assert_eq!(options.endpoint, configured_vlm_endpoint());
        assert_eq!(options.model, configured_vlm_model());
        assert_eq!(options.api_key, configured_vlm_api_key());
        assert_ne!(options.api_key, "request-secret");
    }

    #[test]
    fn base64_encode_known() {
        assert_eq!(base64_encode(b"Hello"), "SGVsbG8=");
    }

    #[test]
    fn parses_layout_embedded_in_model_prose() {
        let json = extract_json(
            "Here is the result: {\"outer_boundary\":[[0,0],[1,0],[0,1]],\"rooms\":[]}",
        )
        .unwrap();
        let layout: FloorplanLayout = serde_json::from_str(json).unwrap();
        validate_layout(&layout).unwrap();
    }

    #[test]
    fn regularizes_noisy_rotated_footprint_to_best_fit_right_angles() {
        let angle = 23f64.to_radians();
        let rotate = |point: [f64; 2]| rotate_2d(point, angle);
        let boundary = [
            rotate([0.02, -0.01]),
            rotate([4.01, 0.04]),
            rotate([3.96, 2.02]),
            rotate([-0.03, 1.97]),
        ];
        let regularized = regularize_boundary(&boundary);
        assert_eq!(regularized.len(), 4);
        for index in 0..4 {
            let previous = regularized[(index + 3) % 4];
            let current = regularized[index];
            let next = regularized[(index + 1) % 4];
            assert!((corner_angle_degrees(previous, current, next) - 90.0).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn missing_file_errors_before_network_call() {
        assert!(extract_floorplan_layout("nonexistent.png").await.is_err());
    }
}
