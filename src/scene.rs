use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
};

use vwm_core::GeometryAnalysis;

pub const ANALYSIS_VERSION: &str = "scene-analysis-v1";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PointCloud {
    pub points: Vec<Point>,
    pub faces: Vec<[usize; 3]>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GeometryMode {
    #[default]
    PointCloud,
    Mesh,
    Mixed,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TextureBinding {
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub role: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MaterialBinding {
    #[serde(default)]
    pub material_name: String,
    #[serde(default)]
    pub texture_file: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CoordinateSystemMetadata {
    #[serde(default = "default_up_axis")]
    pub up_axis: String,
    #[serde(default = "default_units")]
    pub units: String,
}

impl Default for CoordinateSystemMetadata {
    fn default() -> Self {
        Self {
            up_axis: default_up_axis(),
            units: default_units(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TransformRecord {
    pub sequence: u64,
    pub kind: String,
    pub source_plane_id: Option<usize>,
    pub rotation: [[f64; 3]; 3],
    pub translation: [f64; 3],
    pub reversible: bool,
    pub analysis_version: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ScenePackageMetadata {
    #[serde(default)]
    pub base_name: String,
    #[serde(default)]
    pub source_model: Option<String>,
    #[serde(default)]
    pub geometry_mode: GeometryMode,
    #[serde(default)]
    pub texture_bindings: Vec<TextureBinding>,
    #[serde(default)]
    pub material_bindings: Vec<MaterialBinding>,
    #[serde(default)]
    pub coordinate_system: CoordinateSystemMetadata,
    #[serde(default)]
    pub transform_history: Vec<TransformRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GeometryDisposition {
    Keep,
    Remove,
    Uncertain,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct FloatingMeshSettings {
    pub connection_scale: f64,
    pub maximum_scene_point_ratio: f64,
    pub maximum_primary_point_ratio: f64,
    pub maximum_extent_ratio: f64,
    pub minimum_separation_ratio: f64,
}

impl Default for FloatingMeshSettings {
    fn default() -> Self {
        Self {
            connection_scale: 1.0,
            maximum_scene_point_ratio: 0.0125,
            maximum_primary_point_ratio: 0.08,
            maximum_extent_ratio: 0.22,
            minimum_separation_ratio: 0.12,
        }
    }
}

impl FloatingMeshSettings {
    pub fn validate(self) -> Result<Self> {
        for (name, value, minimum, maximum) in [
            ("connection_scale", self.connection_scale, 0.25, 4.0),
            (
                "maximum_scene_point_ratio",
                self.maximum_scene_point_ratio,
                0.001,
                0.25,
            ),
            (
                "maximum_primary_point_ratio",
                self.maximum_primary_point_ratio,
                0.01,
                0.5,
            ),
            ("maximum_extent_ratio", self.maximum_extent_ratio, 0.01, 1.0),
            (
                "minimum_separation_ratio",
                self.minimum_separation_ratio,
                0.0,
                1.0,
            ),
        ] {
            if !value.is_finite() || value < minimum || value > maximum {
                bail!("Floating-mesh {name} must be between {minimum} and {maximum}");
            }
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ThresholdValue {
    pub name: String,
    pub value: f64,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct SupportPlaneScore {
    pub inlier_score: f64,
    pub horizontal_score: f64,
    pub extent_score: f64,
    pub connectedness_score: f64,
    pub elevation_score: f64,
    pub vertical_relationship_score: f64,
    pub noise_clearance_score: f64,
    pub stability_score: f64,
    pub total: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ClusterSummary {
    pub id: usize,
    pub point_count: usize,
    pub centroid: [f64; 3],
    pub extent: [f64; 3],
    pub disposition: GeometryDisposition,
    pub confidence: f64,
    pub score: f64,
    pub reason: String,
    pub method: String,
    pub thresholds: Vec<ThresholdValue>,
    pub is_primary_structural: bool,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct ArtifactSummary {
    pub keep_clusters: usize,
    pub remove_clusters: usize,
    pub uncertain_clusters: usize,
    pub keep_points: usize,
    pub remove_points: usize,
    pub uncertain_points: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct PlaneDetection {
    pub id: usize,
    pub normal: [f64; 3],
    pub centroid: [f64; 3],
    pub inliers: usize,
    pub tolerance: f64,
    pub orientation: String,
    pub average_y: f64,
    pub extent: [f64; 2],
    pub source_cluster_ids: Vec<usize>,
    pub support_rank: usize,
    pub support_score: SupportPlaneScore,
    pub confidence: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct RecognizedObject {
    pub kind: String,
    pub point_count: usize,
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_plane_id: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_cluster_id: Option<usize>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct OrientedBoxPrimitive {
    pub id: usize,
    pub kind: String,
    pub center: [f64; 3],
    pub half_extents: [f64; 3],
    pub axes: [[f64; 3]; 3],
    pub confidence: f64,
    pub attribution_method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_plane_id: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_cluster_id: Option<usize>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CleanedViewSummary {
    pub mode: String,
    pub kept_points: usize,
    pub kept_faces: usize,
    pub hidden_remove_candidates: usize,
    pub highlighted_uncertain_candidates: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ProvenanceSummary {
    pub source_geometry: String,
    pub analysis_version: String,
    pub operation_sequence: u64,
    pub reversible: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SceneAnalysisContext {
    pub analysis_version: String,
    pub point_count: usize,
    pub face_count: usize,
    pub geometry_mode: GeometryMode,
    pub package: ScenePackageMetadata,
    pub floating_mesh_settings: FloatingMeshSettings,
    pub clusters: Vec<ClusterSummary>,
    pub artifact_summary: ArtifactSummary,
    pub planes: Vec<PlaneDetection>,
    pub selected_support_plane_id: Option<usize>,
    pub selected_support_plane_rationale: Option<String>,
    pub objects: Vec<RecognizedObject>,
    pub primitives: Vec<OrientedBoxPrimitive>,
    pub active_cleaned_view: CleanedViewSummary,
    pub transform_history: Vec<TransformRecord>,
    pub provenance: ProvenanceSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vwm_geometry_analysis: Option<GeometryAnalysis>,
    #[serde(skip)]
    cluster_by_point: Vec<usize>,
    #[serde(skip)]
    plane_models: Vec<PlaneModel>,
}

pub type CloudAnalysis = SceneAnalysisContext;

impl SceneAnalysisContext {
    /// Points in clusters classified as removable artifacts are excluded from reconstruction.
    pub fn reconstruction_keep_mask(&self) -> Vec<bool> {
        let removable_cluster_ids = self
            .clusters
            .iter()
            .filter(|cluster| cluster.disposition == GeometryDisposition::Remove)
            .map(|cluster| cluster.id)
            .collect::<HashSet<_>>();
        self.cluster_by_point
            .iter()
            .map(|cluster_id| !removable_cluster_ids.contains(cluster_id))
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct FlatSurfaceResult {
    pub plane: PlaneDetection,
    pub corrected_points: usize,
    pub selected_support_plane_id: usize,
    pub transform: TransformRecord,
    pub package: ScenePackageMetadata,
    pub analysis: SceneAnalysisContext,
}

pub struct LevelledScene {
    pub cloud: PointCloud,
    pub result: FlatSurfaceResult,
}

#[derive(Clone, Debug, PartialEq)]
struct ClusterComponent {
    point_indices: Vec<usize>,
    centroid: [f64; 3],
    minimum: [f64; 3],
    maximum: [f64; 3],
    density: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct Plane {
    normal: [f64; 3],
    offset: f64,
    tolerance: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct PlaneModel {
    plane: Plane,
    inliers: Vec<usize>,
    centroid: [f64; 3],
    area: f64,
    summary: PlaneDetection,
}

pub fn infer_geometry_mode(cloud: &PointCloud) -> GeometryMode {
    if cloud.faces.is_empty() {
        GeometryMode::PointCloud
    } else {
        GeometryMode::Mesh
    }
}

pub fn build_analysis_context(
    cloud: &PointCloud,
    package: ScenePackageMetadata,
) -> Result<SceneAnalysisContext> {
    build_analysis_context_with_settings(cloud, package, FloatingMeshSettings::default())
}

pub fn build_analysis_context_with_settings(
    cloud: &PointCloud,
    mut package: ScenePackageMetadata,
    floating_mesh_settings: FloatingMeshSettings,
) -> Result<SceneAnalysisContext> {
    let floating_mesh_settings = floating_mesh_settings.validate()?;
    if cloud.points.len() < 3 {
        bail!("At least three points are required for scene analysis");
    }

    package.geometry_mode = infer_geometry_mode(cloud);
    if package.base_name.trim().is_empty() {
        package.base_name = "model".to_string();
    }

    let components = detect_components(&cloud.points, floating_mesh_settings.connection_scale)?;
    let primary_id = select_primary_cluster(&components).context("No structural cluster found")?;
    let clusters = classify_clusters(
        &components,
        primary_id,
        cloud.points.len(),
        floating_mesh_settings,
    );
    let artifact_summary = build_artifact_summary(&clusters);
    let cluster_by_point = build_cluster_index(cloud.points.len(), &components);
    let stable_indices = stable_indices(&clusters, &components);
    let kept_faces = count_kept_faces(cloud, &clusters, &cluster_by_point);
    let mut plane_models =
        detect_plane_models(&cloud.points, &stable_indices, &cluster_by_point, &clusters)?;
    rank_support_planes(
        &cloud.points,
        &mut plane_models,
        &clusters,
        primary_id,
        &cluster_by_point,
        &stable_indices,
    );

    let selected_support_plane_id = plane_models.first().map(|plane| plane.summary.id);
    let selected_support_plane_rationale = plane_models.first().map(|plane| {
        format!(
            "Plane {} ranked first with {:.2} total support score ({:.0}% horizontal, {:.0}% connected to the primary cluster, {:.0}% sampling stability).",
            plane.summary.id,
            plane.summary.support_score.total,
            plane.summary.support_score.horizontal_score * 100.0,
            plane.summary.support_score.connectedness_score * 100.0,
            plane.summary.support_score.stability_score * 100.0
        )
    });

    let planes = plane_models
        .iter()
        .map(|plane| plane.summary.clone())
        .collect::<Vec<_>>();
    let objects = derive_objects(
        &clusters,
        &plane_models,
        selected_support_plane_id,
        &stable_indices,
    );
    let primitives = derive_box_primitives(&components, &clusters, &plane_models);
    let operation_sequence = package
        .transform_history
        .last()
        .map(|item| item.sequence)
        .unwrap_or(0);
    let source_geometry = package
        .source_model
        .clone()
        .unwrap_or_else(|| package.base_name.clone());
    let transform_history = package.transform_history.clone();
    let hidden_remove_candidates = artifact_summary.remove_points;
    let highlighted_uncertain_candidates = artifact_summary.uncertain_points;

    Ok(SceneAnalysisContext {
        analysis_version: ANALYSIS_VERSION.to_string(),
        point_count: cloud.points.len(),
        face_count: cloud.faces.len(),
        geometry_mode: package.geometry_mode.clone(),
        package,
        floating_mesh_settings,
        clusters,
        artifact_summary,
        planes,
        selected_support_plane_id,
        selected_support_plane_rationale,
        objects,
        primitives,
        active_cleaned_view: CleanedViewSummary {
            mode: "analysis_preview".to_string(),
            kept_points: stable_indices.len(),
            kept_faces,
            hidden_remove_candidates,
            highlighted_uncertain_candidates,
        },
        transform_history,
        provenance: ProvenanceSummary {
            source_geometry,
            analysis_version: ANALYSIS_VERSION.to_string(),
            operation_sequence,
            reversible: true,
        },
        vwm_geometry_analysis: None,
        cluster_by_point,
        plane_models,
    })
}

pub fn level_scene(cloud: &PointCloud, package: ScenePackageMetadata) -> Result<LevelledScene> {
    let mut analysis = build_analysis_context(cloud, package)?;
    let plane_model = analysis.plane_models.first().cloned().context(
        "No stable support plane found; load geometry with at least one planar structural surface",
    )?;
    let minimum_inliers = (analysis.point_count / 50).max(3);
    if plane_model.inliers.len() < minimum_inliers {
        bail!(
            "Support plane has only {} inliers; at least {} are needed",
            plane_model.inliers.len(),
            minimum_inliers
        );
    }
    if plane_model.summary.support_score.horizontal_score < 0.55 {
        bail!("No credible support plane was found; the strongest candidate is not horizontal enough to level safely");
    }

    let rotation = rotation_to_up(plane_model.plane.normal);
    let translation = [0.0, plane_model.plane.offset, 0.0];
    let corrected_cloud = PointCloud {
        points: cloud
            .points
            .iter()
            .copied()
            .map(|point| transform_point(point, rotation, translation))
            .collect(),
        faces: cloud.faces.clone(),
    };
    let transform = TransformRecord {
        sequence: analysis.transform_history.len() as u64 + 1,
        kind: "level_support_plane".to_string(),
        source_plane_id: Some(plane_model.summary.id),
        rotation,
        translation,
        reversible: true,
        analysis_version: ANALYSIS_VERSION.to_string(),
    };

    analysis.transform_history.push(transform.clone());
    analysis.package.transform_history = analysis.transform_history.clone();
    analysis.active_cleaned_view.mode = "levelled_preview".to_string();
    analysis.provenance.operation_sequence = transform.sequence;

    Ok(LevelledScene {
        cloud: corrected_cloud,
        result: FlatSurfaceResult {
            plane: plane_model.summary.clone(),
            corrected_points: cloud.points.len(),
            selected_support_plane_id: plane_model.summary.id,
            transform,
            package: analysis.package.clone(),
            analysis,
        },
    })
}

fn detect_components(points: &[Point], connection_scale: f64) -> Result<Vec<ClusterComponent>> {
    let (_, _, diagonal) = point_bounds(points);
    let voxel = component_voxel_size(diagonal, points.len()) * connection_scale;
    let mut bins: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
    for (index, point) in points.iter().enumerate() {
        bins.entry(voxel_key(*point, voxel))
            .or_default()
            .push(index);
    }

    let mut visited = vec![false; points.len()];
    let mut components = Vec::new();

    for start in 0..points.len() {
        if visited[start] {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        let mut point_indices = Vec::new();
        let mut minimum = [f64::INFINITY; 3];
        let mut maximum = [f64::NEG_INFINITY; 3];
        let mut centroid_sum = [0.0; 3];
        visited[start] = true;

        while let Some(current) = queue.pop_front() {
            point_indices.push(current);
            let point = points[current];
            centroid_sum[0] += point.x;
            centroid_sum[1] += point.y;
            centroid_sum[2] += point.z;
            for (axis, value) in [point.x, point.y, point.z].into_iter().enumerate() {
                minimum[axis] = minimum[axis].min(value);
                maximum[axis] = maximum[axis].max(value);
            }

            let cell = voxel_key(point, voxel);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        let neighbor = (cell.0 + dx, cell.1 + dy, cell.2 + dz);
                        if let Some(indices) = bins.get(&neighbor) {
                            for &candidate in indices {
                                if visited[candidate] {
                                    continue;
                                }
                                if distance(point, points[candidate]) <= voxel * 2.5 {
                                    visited[candidate] = true;
                                    queue.push_back(candidate);
                                }
                            }
                        }
                    }
                }
            }
        }

        let count = point_indices.len() as f64;
        let centroid = [
            centroid_sum[0] / count.max(1.0),
            centroid_sum[1] / count.max(1.0),
            centroid_sum[2] / count.max(1.0),
        ];
        let extent = [
            (maximum[0] - minimum[0]).max(voxel),
            (maximum[1] - minimum[1]).max(voxel),
            (maximum[2] - minimum[2]).max(voxel),
        ];
        let volume = (extent[0] * extent[1] * extent[2]).max(voxel.powi(3));
        components.push(ClusterComponent {
            point_indices,
            centroid,
            minimum,
            maximum,
            density: count / volume,
        });
    }

    components.sort_by(|left, right| {
        right
            .point_indices
            .len()
            .cmp(&left.point_indices.len())
            .then_with(|| cmp_f64_desc(cluster_diagonal(right), cluster_diagonal(left)))
            .then_with(|| {
                left.point_indices
                    .first()
                    .copied()
                    .unwrap_or_default()
                    .cmp(&right.point_indices.first().copied().unwrap_or_default())
            })
    });

    Ok(components)
}

fn select_primary_cluster(components: &[ClusterComponent]) -> Option<usize> {
    (!components.is_empty()).then_some(0)
}

fn classify_clusters(
    components: &[ClusterComponent],
    primary_id: usize,
    total_points: usize,
    settings: FloatingMeshSettings,
) -> Vec<ClusterSummary> {
    let primary = &components[primary_id];
    let primary_count = primary.point_indices.len() as f64;
    let primary_diagonal = cluster_diagonal(primary).max(1e-6);
    let primary_density = primary.density.max(1e-6);
    let remove_limit = (total_points as f64 * settings.maximum_scene_point_ratio).max(8.0);
    let keep_limit = (total_points as f64 / 16.0).max(24.0);

    components
        .iter()
        .enumerate()
        .map(|(id, component)| {
            let point_count = component.point_indices.len();
            let point_ratio = point_count as f64 / primary_count.max(1.0);
            let extent_ratio = cluster_diagonal(component) / primary_diagonal;
            let centroid_gap =
                distance_to_box(component.centroid, primary.minimum, primary.maximum)
                    / primary_diagonal;
            let density_ratio = (component.density / primary_density).min(1.0);
            let thresholds = vec![
                ThresholdValue {
                    name: "connection_scale".to_string(),
                    value: settings.connection_scale,
                },
                ThresholdValue {
                    name: "remove_limit_points".to_string(),
                    value: remove_limit,
                },
                ThresholdValue {
                    name: "remove_scene_point_ratio".to_string(),
                    value: settings.maximum_scene_point_ratio,
                },
                ThresholdValue {
                    name: "remove_primary_point_ratio".to_string(),
                    value: settings.maximum_primary_point_ratio,
                },
                ThresholdValue {
                    name: "keep_limit_points".to_string(),
                    value: keep_limit,
                },
                ThresholdValue {
                    name: "remove_extent_ratio".to_string(),
                    value: settings.maximum_extent_ratio,
                },
                ThresholdValue {
                    name: "remove_centroid_gap".to_string(),
                    value: settings.minimum_separation_ratio,
                },
            ];

            if id == primary_id {
                return ClusterSummary {
                    id,
                    point_count,
                    centroid: component.centroid,
                    extent: cluster_extent(component),
                    disposition: GeometryDisposition::Keep,
                    confidence: 1.0,
                    score: 1.0,
                    reason: "Largest connected structural cluster retained as the scene backbone."
                        .to_string(),
                    method: "voxel_connected_components".to_string(),
                    thresholds,
                    is_primary_structural: true,
                };
            }

            let (disposition, score, confidence, reason) = if point_count as f64 <= remove_limit
                && point_ratio < settings.maximum_primary_point_ratio
                && extent_ratio < settings.maximum_extent_ratio
                && centroid_gap > settings.minimum_separation_ratio
            {
                (
                    GeometryDisposition::Remove,
                    (1.0 - point_ratio * 2.0 - extent_ratio * 1.5).clamp(0.0, 1.0),
                    (0.65 + centroid_gap * 0.35).clamp(0.0, 1.0),
                    "Detached cluster meets the configured floating-mesh sensitivity thresholds."
                        .to_string(),
                )
            } else if point_count as f64 >= keep_limit
                || point_ratio >= 0.22
                || extent_ratio >= 0.35
                || centroid_gap <= 0.04
            {
                (
                    GeometryDisposition::Keep,
                    (point_ratio + extent_ratio + (1.0 - centroid_gap)).clamp(0.0, 1.0),
                    (0.6 + point_ratio * 0.4 + density_ratio * 0.1).clamp(0.0, 1.0),
                    "Cluster is too large, too connected, or too spatially relevant to remove."
                        .to_string(),
                )
            } else {
                (
                    GeometryDisposition::Uncertain,
                    0.5,
                    (0.45 + density_ratio * 0.2).clamp(0.0, 1.0),
                    "Cluster is detached but not small enough to discard deterministically."
                        .to_string(),
                )
            };

            ClusterSummary {
                id,
                point_count,
                centroid: component.centroid,
                extent: cluster_extent(component),
                disposition,
                confidence,
                score,
                reason,
                method: "voxel_connected_components".to_string(),
                thresholds,
                is_primary_structural: false,
            }
        })
        .collect()
}

fn build_artifact_summary(clusters: &[ClusterSummary]) -> ArtifactSummary {
    let mut summary = ArtifactSummary::default();
    for cluster in clusters {
        match cluster.disposition {
            GeometryDisposition::Keep => {
                summary.keep_clusters += 1;
                summary.keep_points += cluster.point_count;
            }
            GeometryDisposition::Remove => {
                summary.remove_clusters += 1;
                summary.remove_points += cluster.point_count;
            }
            GeometryDisposition::Uncertain => {
                summary.uncertain_clusters += 1;
                summary.uncertain_points += cluster.point_count;
            }
        }
    }
    summary
}

fn build_cluster_index(point_count: usize, components: &[ClusterComponent]) -> Vec<usize> {
    let mut cluster_by_point = vec![0usize; point_count];
    for (cluster_id, component) in components.iter().enumerate() {
        for index in &component.point_indices {
            cluster_by_point[*index] = cluster_id;
        }
    }
    cluster_by_point
}

fn stable_indices(clusters: &[ClusterSummary], components: &[ClusterComponent]) -> Vec<usize> {
    let mut kept = Vec::new();
    for cluster in clusters {
        if cluster.disposition != GeometryDisposition::Remove {
            kept.extend_from_slice(&components[cluster.id].point_indices);
        }
    }
    kept.sort_unstable();
    kept
}

fn count_kept_faces(
    cloud: &PointCloud,
    clusters: &[ClusterSummary],
    cluster_by_point: &[usize],
) -> usize {
    if cloud.faces.is_empty() {
        return 0;
    }
    let removable = clusters
        .iter()
        .filter(|cluster| cluster.disposition == GeometryDisposition::Remove)
        .map(|cluster| cluster.id)
        .collect::<HashSet<_>>();
    cloud
        .faces
        .iter()
        .filter(|face| {
            face.iter()
                .all(|index| !removable.contains(&cluster_by_point[*index]))
        })
        .count()
}

fn detect_plane_models(
    points: &[Point],
    stable_indices: &[usize],
    cluster_by_point: &[usize],
    clusters: &[ClusterSummary],
) -> Result<Vec<PlaneModel>> {
    let mut remaining = stable_indices.to_vec();
    let mut models = Vec::new();
    let minimum_inliers = (stable_indices.len() / 60).max(3);

    for plane_id in 0..6 {
        let Some((plane, inliers)) = find_best_plane(points, &remaining)? else {
            break;
        };
        if inliers.len() < minimum_inliers {
            break;
        }
        let average_y = average_axis(points, &inliers, 1);
        let centroid = average_point(points, &inliers);
        let extent = projected_extent(points, &inliers, plane.normal);
        let area = (extent[0] * extent[1]).max(plane.tolerance.powi(2));
        let mut source_cluster_ids = inliers
            .iter()
            .map(|index| cluster_by_point[*index])
            .collect::<Vec<_>>();
        source_cluster_ids.sort_unstable();
        source_cluster_ids.dedup();
        let summary = PlaneDetection {
            id: plane_id,
            normal: plane.normal,
            centroid,
            inliers: inliers.len(),
            tolerance: plane.tolerance,
            orientation: plane_orientation(plane.normal).to_string(),
            average_y,
            extent,
            source_cluster_ids,
            support_rank: 0,
            support_score: SupportPlaneScore::default(),
            confidence: 0.0,
        };
        models.push(PlaneModel {
            plane,
            inliers: inliers.clone(),
            centroid,
            area,
            summary,
        });
        let inlier_set = inliers.into_iter().collect::<HashSet<_>>();
        remaining.retain(|index| !inlier_set.contains(index));
    }

    if models.is_empty() && !clusters.is_empty() {
        bail!("No planar candidates survived the structural analysis pass");
    }
    Ok(models)
}

fn rank_support_planes(
    points: &[Point],
    planes: &mut [PlaneModel],
    clusters: &[ClusterSummary],
    primary_id: usize,
    cluster_by_point: &[usize],
    stable_indices: &[usize],
) {
    if planes.is_empty() {
        return;
    }
    let max_inliers = planes
        .iter()
        .map(|plane| plane.inliers.len() as f64)
        .fold(1.0, f64::max);
    let max_area = planes.iter().map(|plane| plane.area).fold(1.0, f64::max);
    let primary = &clusters[primary_id];
    let primary_diagonal = magnitude(primary.extent).max(1e-6);
    let remove_centroids = clusters
        .iter()
        .filter(|cluster| cluster.disposition == GeometryDisposition::Remove)
        .map(|cluster| cluster.centroid)
        .collect::<Vec<_>>();
    let horizontal_y = planes
        .iter()
        .filter(|plane| plane.summary.orientation == "horizontal")
        .map(|plane| plane.summary.average_y)
        .collect::<Vec<_>>();
    let min_horizontal_y = horizontal_y.iter().copied().fold(f64::INFINITY, f64::min);
    let max_horizontal_y = horizontal_y
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let vertical_averages = planes
        .iter()
        .filter(|plane| plane.summary.orientation == "vertical")
        .map(|plane| plane.summary.average_y)
        .collect::<Vec<_>>();

    for plane in planes.iter_mut() {
        let horizontal = plane.summary.normal[1].abs().clamp(0.0, 1.0);
        let inlier_score = (plane.inliers.len() as f64 / max_inliers).clamp(0.0, 1.0);
        let horizontal_score = ((horizontal - 0.2) / 0.8).clamp(0.0, 1.0);
        let extent_score = (plane.area / max_area).sqrt().clamp(0.0, 1.0);
        let connectedness_score = plane
            .inliers
            .iter()
            .filter(|index| cluster_by_point[**index] == primary_id)
            .count() as f64
            / plane.inliers.len().max(1) as f64;
        let elevation_score = if plane.summary.orientation == "horizontal"
            && (max_horizontal_y - min_horizontal_y).abs() > plane.summary.tolerance
        {
            (1.0 - ((plane.summary.average_y - min_horizontal_y)
                / (max_horizontal_y - min_horizontal_y)))
                .clamp(0.0, 1.0)
        } else if plane.summary.orientation == "horizontal" {
            1.0
        } else {
            0.0
        };
        let vertical_relationship_score = if vertical_averages.is_empty() {
            0.5
        } else {
            vertical_averages
                .iter()
                .filter(|average| {
                    plane.summary.average_y <= **average + plane.summary.tolerance * 4.0
                })
                .count() as f64
                / vertical_averages.len() as f64
        };
        let noise_clearance_score = if remove_centroids.is_empty() {
            1.0
        } else {
            remove_centroids
                .iter()
                .map(|centroid| distance_xyz(*centroid, plane.centroid) / primary_diagonal)
                .fold(f64::INFINITY, f64::min)
                .clamp(0.0, 0.5)
                / 0.5
        };
        let stability_score =
            plane_sampling_stability(points, &plane.plane, stable_indices, plane.inliers.len());
        let total = (inlier_score * 0.16)
            + (horizontal_score * 0.24)
            + (extent_score * 0.12)
            + (connectedness_score * 0.14)
            + (elevation_score * 0.14)
            + (vertical_relationship_score * 0.08)
            + (noise_clearance_score * 0.05)
            + (stability_score * 0.07);

        plane.summary.support_score = SupportPlaneScore {
            inlier_score,
            horizontal_score,
            extent_score,
            connectedness_score,
            elevation_score,
            vertical_relationship_score,
            noise_clearance_score,
            stability_score,
            total,
        };
        plane.summary.confidence = (total * 0.8 + stability_score * 0.2).clamp(0.0, 1.0);
    }

    planes.sort_by(|left, right| {
        cmp_f64_desc(
            right.summary.support_score.total,
            left.summary.support_score.total,
        )
        .then_with(|| {
            cmp_f64_desc(
                right.summary.support_score.horizontal_score,
                left.summary.support_score.horizontal_score,
            )
        })
        .then_with(|| cmp_f64_asc(left.summary.average_y, right.summary.average_y))
        .then_with(|| right.summary.inliers.cmp(&left.summary.inliers))
        .then_with(|| left.summary.id.cmp(&right.summary.id))
    });

    for (rank, plane) in planes.iter_mut().enumerate() {
        plane.summary.support_rank = rank + 1;
    }
}

fn derive_box_primitives(
    components: &[ClusterComponent],
    clusters: &[ClusterSummary],
    planes: &[PlaneModel],
) -> Vec<OrientedBoxPrimitive> {
    let mut primitives = Vec::with_capacity(components.len() + planes.len());
    for (id, component) in components.iter().enumerate() {
        let summary = &clusters[id];
        let disposition = match summary.disposition {
            GeometryDisposition::Keep => "keep",
            GeometryDisposition::Remove => "remove_candidate",
            GeometryDisposition::Uncertain => "uncertain_candidate",
        };
        let extent = cluster_extent(component);
        primitives.push(OrientedBoxPrimitive {
            id,
            kind: format!("connected_component_box:{disposition}"),
            center: component.centroid,
            half_extents: [extent[0] * 0.5, extent[1] * 0.5, extent[2] * 0.5],
            axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            confidence: summary.confidence,
            attribution_method: "voxel_connected_components".to_string(),
            source_plane_id: None,
            source_cluster_id: Some(summary.id),
        });
    }
    for plane in planes {
        let (u, v) = plane_basis(plane.plane.normal);
        primitives.push(OrientedBoxPrimitive {
            id: primitives.len(),
            kind: "structural_surface_box".to_string(),
            center: plane.centroid,
            half_extents: [
                plane.summary.extent[0] * 0.5,
                plane.summary.extent[1] * 0.5,
                plane.plane.tolerance.max(1e-4),
            ],
            axes: [u, v, plane.plane.normal],
            confidence: plane.summary.confidence,
            attribution_method: "plane_inlier_fit".to_string(),
            source_plane_id: Some(plane.summary.id),
            source_cluster_id: plane.summary.source_cluster_ids.first().copied(),
        });
    }
    primitives
}

fn derive_objects(
    clusters: &[ClusterSummary],
    planes: &[PlaneModel],
    selected_support_plane_id: Option<usize>,
    stable_indices: &[usize],
) -> Vec<RecognizedObject> {
    let selected_plane = selected_support_plane_id.and_then(|selected| {
        planes
            .iter()
            .find(|plane| plane.summary.id == selected)
            .map(|plane| plane.summary.clone())
    });
    let selected_y = selected_plane
        .as_ref()
        .map(|plane| plane.average_y)
        .unwrap_or(0.0);
    let selected_tolerance = selected_plane
        .as_ref()
        .map(|plane| plane.tolerance)
        .unwrap_or(0.01);
    let mut objects = Vec::new();
    let mut inlier_total = 0usize;

    for plane in planes {
        let kind = match plane.summary.orientation.as_str() {
            "horizontal" if Some(plane.summary.id) == selected_support_plane_id => "floor_surface",
            "horizontal" if plane.summary.average_y > selected_y + selected_tolerance * 4.0 => {
                "ceiling_surface"
            }
            "horizontal" => "horizontal_surface",
            "vertical" => "wall_or_panel",
            _ => "sloped_surface",
        };
        inlier_total += plane.summary.inliers;
        objects.push(RecognizedObject {
            kind: kind.to_string(),
            point_count: plane.summary.inliers,
            confidence: plane.summary.confidence,
            source_plane_id: Some(plane.summary.id),
            source_cluster_id: plane.summary.source_cluster_ids.first().copied(),
        });
    }

    if stable_indices.len() > inlier_total {
        objects.push(RecognizedObject {
            kind: "unclassified_geometry".to_string(),
            point_count: stable_indices.len() - inlier_total,
            confidence: ((stable_indices.len() - inlier_total) as f64
                / stable_indices.len().max(1) as f64)
                .clamp(0.0, 1.0),
            source_plane_id: None,
            source_cluster_id: None,
        });
    }

    for cluster in clusters {
        match cluster.disposition {
            GeometryDisposition::Remove => objects.push(RecognizedObject {
                kind: "floating_artifact_candidate".to_string(),
                point_count: cluster.point_count,
                confidence: cluster.confidence,
                source_plane_id: None,
                source_cluster_id: Some(cluster.id),
            }),
            GeometryDisposition::Uncertain => objects.push(RecognizedObject {
                kind: "uncertain_artifact_cluster".to_string(),
                point_count: cluster.point_count,
                confidence: cluster.confidence,
                source_plane_id: None,
                source_cluster_id: Some(cluster.id),
            }),
            GeometryDisposition::Keep => {}
        }
    }

    objects
}

fn find_best_plane(points: &[Point], candidates: &[usize]) -> Result<Option<(Plane, Vec<usize>)>> {
    if candidates.len() < 3 {
        return Ok(None);
    }
    let tolerance = plane_tolerance(points, candidates);
    let mut best: Option<(Plane, Vec<usize>)> = None;
    let attempts = candidates.len().clamp(96, 320);
    let mut state = 0x9E37_79B9_7F4A_7C15u64;

    for _ in 0..attempts {
        let a = candidates[next_index(&mut state, candidates.len())];
        let b = candidates[next_index(&mut state, candidates.len())];
        let c = candidates[next_index(&mut state, candidates.len())];
        if a == b || a == c || b == c {
            continue;
        }
        let Some(mut plane) = Plane::from_points(points[a], points[b], points[c]) else {
            continue;
        };
        plane.tolerance = tolerance;
        let inliers = candidates
            .iter()
            .copied()
            .filter(|index| plane.distance(points[*index]) <= tolerance)
            .collect::<Vec<_>>();
        if best
            .as_ref()
            .is_none_or(|(_, best_inliers)| inliers.len() > best_inliers.len())
        {
            best = Some((plane, inliers));
        }
    }

    Ok(best)
}

fn component_voxel_size(diagonal: f64, point_count: usize) -> f64 {
    let density_scale = diagonal / (point_count.max(1) as f64).cbrt();
    density_scale.max(diagonal * 0.02).max(0.001)
}

fn voxel_key(point: Point, voxel: f64) -> (i32, i32, i32) {
    (
        (point.x / voxel).floor() as i32,
        (point.y / voxel).floor() as i32,
        (point.z / voxel).floor() as i32,
    )
}

fn plane_tolerance(points: &[Point], indices: &[usize]) -> f64 {
    let (_, _, diagonal) = point_bounds_subset(points, indices);
    (diagonal * 0.005).max(0.000_01)
}

fn point_bounds(points: &[Point]) -> ([f64; 3], [f64; 3], f64) {
    let indices = (0..points.len()).collect::<Vec<_>>();
    point_bounds_subset(points, &indices)
}

fn point_bounds_subset(points: &[Point], indices: &[usize]) -> ([f64; 3], [f64; 3], f64) {
    let mut minimum = [f64::INFINITY; 3];
    let mut maximum = [f64::NEG_INFINITY; 3];
    for index in indices {
        let point = points[*index];
        for (axis, value) in [point.x, point.y, point.z].into_iter().enumerate() {
            minimum[axis] = minimum[axis].min(value);
            maximum[axis] = maximum[axis].max(value);
        }
    }
    let diagonal = ((maximum[0] - minimum[0]).powi(2)
        + (maximum[1] - minimum[1]).powi(2)
        + (maximum[2] - minimum[2]).powi(2))
    .sqrt();
    (minimum, maximum, diagonal)
}

impl Plane {
    fn from_points(a: Point, b: Point, c: Point) -> Option<Self> {
        let ab = [b.x - a.x, b.y - a.y, b.z - a.z];
        let ac = [c.x - a.x, c.y - a.y, c.z - a.z];
        let mut normal = normalize(cross(ab, ac))?;
        if normal[1] < -1e-9
            || (normal[1].abs() <= 1e-9
                && (normal[0] < -1e-9 || (normal[0].abs() <= 1e-9 && normal[2] < 0.0)))
        {
            normal = [-normal[0], -normal[1], -normal[2]];
        }
        Some(Self {
            normal,
            offset: -(normal[0] * a.x + normal[1] * a.y + normal[2] * a.z),
            tolerance: 0.0,
        })
    }

    fn distance(&self, point: Point) -> f64 {
        (self.normal[0] * point.x
            + self.normal[1] * point.y
            + self.normal[2] * point.z
            + self.offset)
            .abs()
    }
}

fn plane_orientation(normal: [f64; 3]) -> &'static str {
    let y = normal[1].abs();
    if y >= 0.85 {
        "horizontal"
    } else if y <= 0.2 {
        "vertical"
    } else {
        "sloped"
    }
}

fn projected_extent(points: &[Point], inliers: &[usize], normal: [f64; 3]) -> [f64; 2] {
    let (u, v) = plane_basis(normal);
    let mut min_u = f64::INFINITY;
    let mut max_u = f64::NEG_INFINITY;
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;

    for index in inliers {
        let point = points[*index];
        let u_value = point.x * u[0] + point.y * u[1] + point.z * u[2];
        let v_value = point.x * v[0] + point.y * v[1] + point.z * v[2];
        min_u = min_u.min(u_value);
        max_u = max_u.max(u_value);
        min_v = min_v.min(v_value);
        max_v = max_v.max(v_value);
    }

    [(max_u - min_u).max(0.0), (max_v - min_v).max(0.0)]
}

fn plane_basis(normal: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let reference = if normal[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let u = normalize(cross(normal, reference)).unwrap_or([1.0, 0.0, 0.0]);
    let v = normalize(cross(normal, u)).unwrap_or([0.0, 0.0, 1.0]);
    (u, v)
}

fn average_axis(points: &[Point], indices: &[usize], axis: usize) -> f64 {
    let total = indices
        .iter()
        .map(|index| match axis {
            0 => points[*index].x,
            1 => points[*index].y,
            _ => points[*index].z,
        })
        .sum::<f64>();
    total / indices.len().max(1) as f64
}

fn average_point(points: &[Point], indices: &[usize]) -> [f64; 3] {
    let mut sum = [0.0; 3];
    for index in indices {
        sum[0] += points[*index].x;
        sum[1] += points[*index].y;
        sum[2] += points[*index].z;
    }
    let count = indices.len().max(1) as f64;
    [sum[0] / count, sum[1] / count, sum[2] / count]
}

fn plane_sampling_stability(
    points: &[Point],
    plane: &Plane,
    stable_indices: &[usize],
    inlier_count: usize,
) -> f64 {
    let sample_step = (stable_indices.len() / 512).max(2);
    let sampled = stable_indices
        .iter()
        .step_by(sample_step)
        .copied()
        .collect::<Vec<_>>();
    if sampled.len() < 3 {
        return 1.0;
    }
    let expected = inlier_count as f64 * sampled.len() as f64 / stable_indices.len().max(1) as f64;
    let actual_inliers = sampled
        .iter()
        .filter(|index| plane.distance(points[**index]) <= plane.tolerance)
        .count();
    if expected <= 1.0 {
        1.0
    } else {
        (actual_inliers as f64 / expected).clamp(0.0, 1.0)
    }
}

fn rotation_to_up(normal: [f64; 3]) -> [[f64; 3]; 3] {
    let target = [0.0, 1.0, 0.0];
    let axis = cross(normal, target);
    let sine = dot(axis, axis).sqrt();
    let cosine = dot(normal, target);
    if sine < 1e-12 {
        return if cosine >= 0.0 {
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        } else {
            [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]]
        };
    }
    let skew = [
        [0.0, -axis[2], axis[1]],
        [axis[2], 0.0, -axis[0]],
        [-axis[1], axis[0], 0.0],
    ];
    let skew_squared = multiply(skew, skew);
    let factor = (1.0 - cosine) / (sine * sine);
    let mut result = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            result[row][column] = (if row == column { 1.0 } else { 0.0 })
                + skew[row][column]
                + skew_squared[row][column] * factor;
        }
    }
    result
}

fn transform_point(point: Point, rotation: [[f64; 3]; 3], translation: [f64; 3]) -> Point {
    let rotated = rotate(point, rotation);
    Point {
        x: rotated.x + translation[0],
        y: rotated.y + translation[1],
        z: rotated.z + translation[2],
    }
}

fn rotate(point: Point, matrix: [[f64; 3]; 3]) -> Point {
    Point {
        x: matrix[0][0] * point.x + matrix[0][1] * point.y + matrix[0][2] * point.z,
        y: matrix[1][0] * point.x + matrix[1][1] * point.y + matrix[1][2] * point.z,
        z: matrix[2][0] * point.x + matrix[2][1] * point.y + matrix[2][2] * point.z,
    }
}

fn multiply(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut result = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            result[row][column] = (0..3).map(|index| a[row][index] * b[index][column]).sum();
        }
    }
    result
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize(vector: [f64; 3]) -> Option<[f64; 3]> {
    let length = dot(vector, vector).sqrt();
    (length > 1e-12).then(|| [vector[0] / length, vector[1] / length, vector[2] / length])
}

fn distance(left: Point, right: Point) -> f64 {
    ((left.x - right.x).powi(2) + (left.y - right.y).powi(2) + (left.z - right.z).powi(2)).sqrt()
}

fn distance_xyz(left: [f64; 3], right: [f64; 3]) -> f64 {
    ((left[0] - right[0]).powi(2) + (left[1] - right[1]).powi(2) + (left[2] - right[2]).powi(2))
        .sqrt()
}

fn distance_to_box(point: [f64; 3], minimum: [f64; 3], maximum: [f64; 3]) -> f64 {
    let dx = if point[0] < minimum[0] {
        minimum[0] - point[0]
    } else if point[0] > maximum[0] {
        point[0] - maximum[0]
    } else {
        0.0
    };
    let dy = if point[1] < minimum[1] {
        minimum[1] - point[1]
    } else if point[1] > maximum[1] {
        point[1] - maximum[1]
    } else {
        0.0
    };
    let dz = if point[2] < minimum[2] {
        minimum[2] - point[2]
    } else if point[2] > maximum[2] {
        point[2] - maximum[2]
    } else {
        0.0
    };
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn cluster_extent(component: &ClusterComponent) -> [f64; 3] {
    [
        component.maximum[0] - component.minimum[0],
        component.maximum[1] - component.minimum[1],
        component.maximum[2] - component.minimum[2],
    ]
}

fn cluster_diagonal(component: &ClusterComponent) -> f64 {
    magnitude(cluster_extent(component))
}

fn magnitude(vector: [f64; 3]) -> f64 {
    (vector[0].powi(2) + vector[1].powi(2) + vector[2].powi(2)).sqrt()
}

fn next_index(state: &mut u64, length: usize) -> usize {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1);
    (*state as usize) % length
}

fn cmp_f64_desc(left: f64, right: f64) -> Ordering {
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

fn cmp_f64_asc(left: f64, right: f64) -> Ordering {
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

fn default_up_axis() -> String {
    "Y".to_string()
}

fn default_units() -> String {
    "scene_units".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_plane(width: usize, depth: usize, y: f64) -> Vec<Point> {
        let mut points = Vec::new();
        for x in 0..width {
            for z in 0..depth {
                points.push(Point {
                    x: x as f64,
                    y,
                    z: z as f64,
                });
            }
        }
        points
    }

    fn sloped_plane(
        width: usize,
        depth: usize,
        base_y: f64,
        slope_x: f64,
        slope_z: f64,
    ) -> Vec<Point> {
        let mut points = Vec::new();
        for x in 0..width {
            for z in 0..depth {
                points.push(Point {
                    x: x as f64,
                    y: base_y + slope_x * x as f64 + slope_z * z as f64,
                    z: z as f64,
                });
            }
        }
        points
    }

    fn wall_plane(x: f64, width: usize, height: usize) -> Vec<Point> {
        let mut points = Vec::new();
        for y in 0..height {
            for z in 0..width {
                points.push(Point {
                    x,
                    y: y as f64,
                    z: z as f64,
                });
            }
        }
        points
    }

    fn detached_cluster(origin: [f64; 3], width: usize, height: usize, depth: usize) -> Vec<Point> {
        let mut points = Vec::new();
        for x in 0..width {
            for y in 0..height {
                for z in 0..depth {
                    points.push(Point {
                        x: origin[0] + x as f64 * 0.5,
                        y: origin[1] + y as f64 * 0.5,
                        z: origin[2] + z as f64 * 0.5,
                    });
                }
            }
        }
        points
    }

    fn scene(points: Vec<Point>) -> PointCloud {
        PointCloud {
            points,
            faces: Vec::new(),
        }
    }

    fn sample_metadata() -> ScenePackageMetadata {
        ScenePackageMetadata {
            base_name: "sample".to_string(),
            source_model: Some("sample.zip".to_string()),
            geometry_mode: GeometryMode::PointCloud,
            texture_bindings: vec![TextureBinding {
                file: "textures/facade.png".to_string(),
                role: "draped_color".to_string(),
            }],
            material_bindings: vec![MaterialBinding {
                material_name: "facade".to_string(),
                texture_file: Some("textures/facade.png".to_string()),
            }],
            coordinate_system: CoordinateSystemMetadata::default(),
            transform_history: Vec::new(),
        }
    }

    fn transpose(matrix: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
        let mut result = [[0.0; 3]; 3];
        for row in 0..3 {
            for column in 0..3 {
                result[row][column] = matrix[column][row];
            }
        }
        result
    }

    #[test]
    fn primary_cluster_selection_prefers_largest_component() {
        let mut points = grid_plane(10, 10, 0.0);
        points.extend(detached_cluster([50.0, 10.0, 50.0], 2, 2, 2));
        let analysis = build_analysis_context(&scene(points), sample_metadata()).unwrap();
        assert!(analysis.clusters[0].is_primary_structural);
        assert_eq!(analysis.clusters[0].point_count, 100);
    }

    #[test]
    fn floating_blob_detection_marks_small_detached_cluster_for_removal() {
        let mut points = grid_plane(10, 10, 0.0);
        points.extend(detached_cluster([50.0, 10.0, 50.0], 2, 1, 2));
        let analysis = build_analysis_context(&scene(points), sample_metadata()).unwrap();
        assert!(analysis
            .clusters
            .iter()
            .any(|cluster| cluster.disposition == GeometryDisposition::Remove));
    }

    #[test]
    fn medium_detached_cluster_is_left_uncertain() {
        let mut points = grid_plane(10, 10, 0.0);
        points.extend(detached_cluster([30.0, 5.0, 30.0], 3, 2, 3));
        let analysis = build_analysis_context(&scene(points), sample_metadata()).unwrap();
        assert!(analysis
            .clusters
            .iter()
            .any(|cluster| cluster.disposition == GeometryDisposition::Uncertain));
    }

    #[test]
    fn support_plane_ranking_returns_multiple_candidates() {
        let mut points = grid_plane(8, 8, 0.0);
        points.extend(grid_plane(7, 7, 4.0));
        points.extend(wall_plane(0.0, 8, 10));
        let analysis = build_analysis_context(&scene(points), sample_metadata()).unwrap();
        assert!(analysis.planes.len() >= 2);
        assert_eq!(analysis.planes[0].support_rank, 1);
        assert!(analysis
            .primitives
            .iter()
            .filter(|primitive| primitive.kind == "structural_surface_box")
            .all(
                |primitive| primitive.attribution_method == "plane_inlier_fit"
                    && primitive.source_plane_id.is_some()
            ));
    }

    #[test]
    fn support_plane_selection_prefers_floor_when_largest_plane_is_a_wall() {
        let mut points = grid_plane(8, 8, 0.0);
        points.extend(wall_plane(0.0, 12, 12));
        points.extend(grid_plane(7, 7, 6.0));
        let analysis = build_analysis_context(&scene(points), sample_metadata()).unwrap();
        let selected = analysis
            .planes
            .iter()
            .find(|plane| Some(plane.id) == analysis.selected_support_plane_id)
            .unwrap();
        assert_eq!(selected.orientation, "horizontal");
        assert!(selected.average_y < 1.0);
    }

    #[test]
    fn levelling_transform_is_reversible() {
        let mut points = sloped_plane(8, 8, 2.0, 0.1, 0.05);
        points.extend(detached_cluster([2.0, 4.0, 2.0], 2, 2, 2));
        let original = scene(points);
        let levelled = level_scene(&original, sample_metadata()).unwrap();
        let inverse_rotation = transpose(levelled.result.transform.rotation);
        let translation = levelled.result.transform.translation;
        for (source, transformed) in original.points.iter().zip(levelled.cloud.points.iter()) {
            let translated = Point {
                x: transformed.x - translation[0],
                y: transformed.y - translation[1],
                z: transformed.z - translation[2],
            };
            let restored = rotate(translated, inverse_rotation);
            assert!(distance(*source, restored) < 1e-6);
        }
    }

    #[test]
    fn texture_and_material_metadata_survive_transforms() {
        let cloud = scene(sloped_plane(6, 6, 1.0, 0.05, 0.02));
        let metadata = sample_metadata();
        let levelled = level_scene(&cloud, metadata.clone()).unwrap();
        assert_eq!(
            levelled.result.package.texture_bindings,
            metadata.texture_bindings
        );
        assert_eq!(
            levelled.result.package.material_bindings,
            metadata.material_bindings
        );
    }

    #[test]
    fn analysis_is_deterministic_for_same_input() {
        let mut points = grid_plane(8, 8, 0.0);
        points.extend(wall_plane(0.0, 10, 10));
        points.extend(detached_cluster([40.0, 5.0, 40.0], 2, 2, 2));
        let cloud = scene(points);
        let first = build_analysis_context(&cloud, sample_metadata()).unwrap();
        let second = build_analysis_context(&cloud, sample_metadata()).unwrap();
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
    }
}
