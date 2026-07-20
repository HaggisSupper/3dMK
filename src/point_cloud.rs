use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use serde_json::json;
use std::{
    collections::HashMap,
    io::{BufWriter, Write},
    path::Path,
    process::{Command, Stdio},
};

use crate::ai_vision::{FloorplanLayout, FloorplanRoom};
use crate::scene::{build_analysis_context, infer_geometry_mode, level_scene};
pub use crate::scene::{
    CloudAnalysis, CoordinateSystemMetadata, FlatSurfaceResult, GeometryMode, MaterialBinding,
    Point, PointCloud, ScenePackageMetadata, TextureBinding,
};
use vwm_core::{CanonicalScene, GeometryAnalysis};
use vwm_geometry::{analyze_scene as analyze_vwm_scene, GeometryConfig};
use vwm_io::load_scene as load_vwm_scene;
use vwm_implicit::{OrientedPointSet, PoissonConfig, PoissonReconstructor};

const MAX_POINTS: usize = 2_000_000;

pub fn pdal_available() -> bool {
    Command::new("pdal")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Reconstructs a mesh with the locally installed PDAL executable.
pub fn run_pdal_pipeline(input_path: &Path, output_path: &Path) -> Result<()> {
    if !input_path.is_file() {
        bail!("Point-cloud input does not exist: {}", input_path.display());
    }
    if !pdal_available() {
        bail!("PDAL is not installed or is not available on PATH");
    }

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let pipeline_path = output_path.with_extension("pipeline.json");
    let pipeline = json!({
        "pipeline": [
            input_path.to_string_lossy(),
            { "type": "filters.normal", "knn": 8 },
            { "type": "filters.poisson", "depth": 10 },
            output_path.to_string_lossy()
        ]
    });
    std::fs::write(&pipeline_path, serde_json::to_vec_pretty(&pipeline)?)?;

    let result = Command::new("pdal")
        .arg("pipeline")
        .arg(&pipeline_path)
        .output()
        .context("Failed to start PDAL")?;
    std::fs::remove_file(&pipeline_path).ok();

    if !result.status.success() {
        bail!(
            "PDAL exited with {}: {}",
            result.status,
            String::from_utf8_lossy(&result.stderr).trim()
        );
    }
    if !output_path.is_file() {
        bail!("PDAL completed without producing {}", output_path.display());
    }

    Ok(())
}

/// Pure-Rust fallback for machines without PDAL. It samples the cloud, estimates
/// local normals, and runs the screened-Poisson engine already shipped in VWM.
pub fn run_implicit_pipeline(input_path: &Path, output_path: &Path) -> Result<()> {
    let cloud = read_ascii_point_cloud(input_path)?;
    if cloud.points.len() < 4 {
        bail!("Point-cloud reconstruction requires at least four points");
    }
    let sample = sample_reconstruction_points(&cloud.points, 120_000);
    let (sample, normals) = estimate_local_normals(&sample)?;
    let oriented = OrientedPointSet::new(
        sample
            .iter()
            .map(|point| [point.x, point.y, point.z])
            .collect(),
        normals,
    )
    .map_err(|error| anyhow!("Could not orient point cloud: {error}"))?;
    let field = PoissonReconstructor
        .reconstruct(
            &oriented,
            PoissonConfig {
                max_depth: 8,
                density_estimation_depth: 5,
                ..PoissonConfig::default()
            },
        )
        .map_err(|error| anyhow!("Rust Poisson reconstruction failed: {error}"))?;
    let mesh = field
        .reconstruct_mesh()
        .map_err(|error| anyhow!("Rust Poisson mesh extraction failed: {error}"))?;
    if mesh.positions.is_empty() || mesh.triangles.is_empty() {
        bail!("Rust Poisson reconstruction produced no triangles");
    }
    let result = PointCloud {
        points: mesh
            .positions
            .into_iter()
            .map(|point| Point {
                x: point[0] as f64,
                y: point[1] as f64,
                z: point[2] as f64,
            })
            .collect(),
        faces: mesh
            .triangles
            .into_iter()
            .map(|face| [face[0] as usize, face[1] as usize, face[2] as usize])
            .collect(),
    };
    write_ascii_ply(output_path, &result)?;
    Ok(())
}

fn sample_reconstruction_points(points: &[Point], limit: usize) -> Vec<Point> {
    if points.len() <= limit {
        return points.to_vec();
    }
    let stride = (points.len() as f64 / limit as f64).ceil() as usize;
    points.iter().step_by(stride.max(1)).cloned().collect()
}

fn estimate_local_normals(points: &[Point]) -> Result<(Vec<Point>, Vec<[f64; 3]>)> {
    if points.len() < 4 {
        bail!("At least four points are required for local normal estimation");
    }
    let mut oriented_points = Vec::with_capacity(points.len());
    let mut normals = Vec::with_capacity(points.len());
    let (minimum, maximum) = point_bounds(points);
    let diagonal = ((maximum[0] - minimum[0]).powi(2)
        + (maximum[1] - minimum[1]).powi(2)
        + (maximum[2] - minimum[2]).powi(2))
    .sqrt();
    let voxel = (diagonal / (points.len() as f64).cbrt() * 2.5).max(1e-6);
    let mut bins: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
    for (index, point) in points.iter().enumerate() {
        bins.entry(reconstruction_voxel_key(*point, voxel))
            .or_default()
            .push(index);
    }
    let centroid = points.iter().fold([0.0; 3], |mut sum, point| {
        sum[0] += point.x;
        sum[1] += point.y;
        sum[2] += point.z;
        sum
    });
    let centroid = [
        centroid[0] / points.len() as f64,
        centroid[1] / points.len() as f64,
        centroid[2] / points.len() as f64,
    ];
    for (index, point) in points.iter().enumerate() {
        let mut nearest = [(f64::INFINITY, 0usize); 3];
        let key = reconstruction_voxel_key(*point, voxel);
        for radius in 1..=4 {
            for dx in -radius..=radius {
                for dy in -radius..=radius {
                    for dz in -radius..=radius {
                        if let Some(candidates) = bins.get(&(key.0 + dx, key.1 + dy, key.2 + dz)) {
                            for candidate_index in candidates {
                                if index == *candidate_index { continue; }
                                let distance = squared_distance(*point, points[*candidate_index]);
                                if distance < nearest[0].0 {
                                    nearest[2] = nearest[1]; nearest[1] = nearest[0]; nearest[0] = (distance, *candidate_index);
                                } else if distance < nearest[1].0 {
                                    nearest[2] = nearest[1]; nearest[1] = (distance, *candidate_index);
                                } else if distance < nearest[2].0 {
                                    nearest[2] = (distance, *candidate_index);
                                }
                            }
                        }
                    }
                }
            }
            if nearest[2].0.is_finite() { break; }
        }
        // ponytail: isolated scan returns are discarded; upgrade to density-weighted confidence when cleanup needs to retain them.
        if !nearest[2].0.is_finite() { continue; }
        let a = points[nearest[0].1];
        let b = points[nearest[1].1];
        let ab = [b.x - a.x, b.y - a.y, b.z - a.z];
        let ac = [
            points[nearest[2].1].x - a.x,
            points[nearest[2].1].y - a.y,
            points[nearest[2].1].z - a.z,
        ];
        let mut normal = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if !length.is_finite() || length <= 1e-12 {
            continue;
        }
        normal.iter_mut().for_each(|component| *component /= length);
        let outward = [point.x - centroid[0], point.y - centroid[1], point.z - centroid[2]];
        if normal[0] * outward[0] + normal[1] * outward[1] + normal[2] * outward[2] < 0.0 {
            normal.iter_mut().for_each(|component| *component = -*component);
        }
        oriented_points.push(*point);
        normals.push(normal);
    }
    if oriented_points.len() < 4 {
        bail!("Normal estimation retained fewer than four supported points");
    }
    Ok((oriented_points, normals))
}

fn squared_distance(left: Point, right: Point) -> f64 {
    (left.x - right.x).powi(2) + (left.y - right.y).powi(2) + (left.z - right.z).powi(2)
}

fn reconstruction_voxel_key(point: Point, voxel: f64) -> (i32, i32, i32) {
    (
        (point.x / voxel).floor() as i32,
        (point.y / voxel).floor() as i32,
        (point.z / voxel).floor() as i32,
    )
}

/// Deterministic structural recognition for ASCII PLY and XYZ point clouds.
pub fn analyze_file(input_path: &Path) -> Result<CloudAnalysis> {
    analyze_file_with_metadata(input_path, None)
}

pub fn analyze_file_with_metadata(
    input_path: &Path,
    package: Option<ScenePackageMetadata>,
) -> Result<CloudAnalysis> {
    let (cloud, geometry_analysis) = load_scene_point_cloud(input_path)?;
    let package = default_scene_package(input_path, &cloud, package);
    let mut analysis = build_analysis_context(&cloud, package)?;
    analysis.vwm_geometry_analysis = geometry_analysis;
    Ok(analysis)
}

/// Levels the selected support plane with a rigid model transform.
pub fn flatten_file(input_path: &Path, output_path: &Path) -> Result<FlatSurfaceResult> {
    flatten_file_with_metadata(input_path, output_path, None)
}

pub fn flatten_file_with_metadata(
    input_path: &Path,
    output_path: &Path,
    package: Option<ScenePackageMetadata>,
) -> Result<FlatSurfaceResult> {
    let (cloud, _) = load_scene_point_cloud(input_path)?;
    let package = default_scene_package(input_path, &cloud, package);
    let levelled = level_scene(&cloud, package)?;
    write_ascii_ply(output_path, &levelled.cloud)?;
    Ok(levelled.result)
}

#[derive(Clone, Debug, Serialize)]
pub struct MeshSmoothResult {
    pub smoothed_points: usize,
    pub iterations: usize,
    pub strength: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ModelFloorplanResult {
    pub layout: FloorplanLayout,
    pub correction: FlatSurfaceResult,
    pub cut_height: f64,
}

pub fn smooth_file(
    input_path: &Path,
    output_path: &Path,
    iterations: usize,
    strength: f64,
) -> Result<MeshSmoothResult> {
    if iterations == 0 {
        bail!("Smoothing iterations must be at least 1");
    }
    if !(0.0..=1.0).contains(&strength) || strength == 0.0 {
        bail!("Smoothing strength must be between 0 and 1");
    }

    let (mut cloud, _) = load_scene_point_cloud(input_path)?;
    if cloud.faces.is_empty() {
        bail!("Mesh smoothing requires triangle faces");
    }

    // ponytail: full-mesh adjacency is fine here; add chunked/tiled smoothing only if release builds choke on very large jobs.
    let mut neighbors = vec![Vec::<usize>::new(); cloud.points.len()];
    for face in &cloud.faces {
        connect_neighbors(&mut neighbors, face[0], face[1]);
        connect_neighbors(&mut neighbors, face[1], face[2]);
        connect_neighbors(&mut neighbors, face[2], face[0]);
    }
    for list in &mut neighbors {
        list.sort_unstable();
        list.dedup();
    }

    for _ in 0..iterations {
        relax_points(&mut cloud.points, &neighbors, strength);
        relax_points(&mut cloud.points, &neighbors, -(strength * 1.02));
    }
    write_ascii_ply(output_path, &cloud)?;

    Ok(MeshSmoothResult {
        smoothed_points: cloud.points.len(),
        iterations,
        strength,
    })
}

pub fn recognize_floorplan_from_model(
    input_path: &Path,
    slice_percent: f64,
    package: Option<ScenePackageMetadata>,
) -> Result<ModelFloorplanResult> {
    let (cloud, _) = load_scene_point_cloud(input_path)?;
    if cloud.faces.is_empty() {
        bail!("Model floorplan recognition requires a mesh with triangle faces");
    }

    let package = default_scene_package(input_path, &cloud, package);
    let levelled = level_scene(&cloud, package)?;
    let (minimum, maximum) = point_bounds(&levelled.cloud.points);
    let span_y = (maximum[1] - minimum[1]).max(0.000_01);
    let slice_percent = slice_percent.clamp(0.0, 1.0);
    let cut_height = minimum[1] + span_y * slice_percent;
    let layout = extract_floorplan_layout(&levelled.cloud, cut_height)?;

    Ok(ModelFloorplanResult {
        layout,
        correction: levelled.result,
        cut_height,
    })
}

fn default_scene_package(
    input_path: &Path,
    cloud: &PointCloud,
    package: Option<ScenePackageMetadata>,
) -> ScenePackageMetadata {
    let mut package = package.unwrap_or_default();
    if package.base_name.trim().is_empty() {
        package.base_name = input_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.trim().is_empty())
            .unwrap_or("model")
            .to_string();
    }
    if package.source_model.is_none() {
        package.source_model = Some(
            input_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("point-cloud.ply")
                .to_string(),
        );
    }
    package.geometry_mode = infer_geometry_mode(cloud);
    if package.coordinate_system.up_axis.trim().is_empty() {
        package.coordinate_system.up_axis = "Y".to_string();
    }
    if package.coordinate_system.units.trim().is_empty() {
        package.coordinate_system.units = "scene_units".to_string();
    }
    package
}

fn read_ascii_point_cloud(input_path: &Path) -> Result<PointCloud> {
    if !input_path.is_file() {
        bail!("Point-cloud input does not exist: {}", input_path.display());
    }
    let text = std::fs::read_to_string(input_path).with_context(|| {
        format!(
            "Only ASCII PLY/XYZ files are supported for analysis: {}",
            input_path.display()
        )
    })?;
    if text.trim_start().starts_with("ply") {
        parse_ascii_ply(&text)
    } else {
        parse_xyz(&text)
    }
}

fn load_scene_point_cloud(input_path: &Path) -> Result<(PointCloud, Option<GeometryAnalysis>)> {
    match read_ascii_point_cloud(input_path) {
        Ok(cloud) => {
            let geometry_analysis = analyze_point_cloud_with_vwm(&cloud);
            Ok((cloud, geometry_analysis))
        }
        Err(ascii_error) => {
            let scene = load_vwm_scene(input_path).with_context(|| {
                format!(
                    "ASCII point-cloud parsing failed for {} ({ascii_error}); VWM scene loading also failed",
                    input_path.display()
                )
            })?;
            let cloud = canonical_scene_to_point_cloud(&scene);
            let geometry_analysis = analyze_canonical_scene(&scene);
            Ok((cloud, geometry_analysis))
        }
    }
}

fn analyze_point_cloud_with_vwm(cloud: &PointCloud) -> Option<GeometryAnalysis> {
    if cloud.faces.is_empty() {
        return None;
    }
    let scene = point_cloud_to_canonical_scene(cloud);
    analyze_canonical_scene(&scene)
}

fn analyze_canonical_scene(scene: &CanonicalScene) -> Option<GeometryAnalysis> {
    if scene
        .indices
        .as_ref()
        .is_none_or(|indices| indices.is_empty())
    {
        return None;
    }
    analyze_vwm_scene(scene, GeometryConfig::default()).ok()
}

fn canonical_scene_to_point_cloud(scene: &CanonicalScene) -> PointCloud {
    let points = scene
        .vertices
        .iter()
        .map(|vertex| Point {
            x: vertex[0] as f64,
            y: vertex[1] as f64,
            z: vertex[2] as f64,
        })
        .collect::<Vec<_>>();
    let faces = scene
        .indices
        .as_ref()
        .map(|indices| {
            indices
                .iter()
                .map(|face| [face[0] as usize, face[1] as usize, face[2] as usize])
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    PointCloud { points, faces }
}

fn point_cloud_to_canonical_scene(cloud: &PointCloud) -> CanonicalScene {
    CanonicalScene {
        vertices: cloud
            .points
            .iter()
            .map(|point| [point.x as f32, point.y as f32, point.z as f32])
            .collect(),
        normals: None,
        indices: (!cloud.faces.is_empty()).then(|| {
            cloud
                .faces
                .iter()
                .map(|face| [face[0] as u32, face[1] as u32, face[2] as u32])
                .collect()
        }),
        colors: None,
        uvs: None,
        material_ids: None,
        mesh: !cloud.faces.is_empty(),
        point_cloud: cloud.faces.is_empty(),
        ..Default::default()
    }
}

fn parse_xyz(text: &str) -> Result<PointCloud> {
    let mut cloud = PointCloud::default();
    for (line_number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let values = line
            .split_whitespace()
            .take(3)
            .map(str::parse::<f64>)
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("Invalid XYZ data on line {}", line_number + 1))?;
        if values.len() != 3 || !values.iter().all(|value| value.is_finite()) {
            bail!(
                "Expected finite X Y Z coordinates on line {}",
                line_number + 1
            );
        }
        cloud.points.push(Point {
            x: values[0],
            y: values[1],
            z: values[2],
        });
        if cloud.points.len() > MAX_POINTS {
            bail!(
                "Point cloud exceeds the {} point analysis limit",
                MAX_POINTS
            );
        }
    }
    if cloud.points.len() < 3 {
        bail!("At least three points are required");
    }
    Ok(cloud)
}

fn parse_ascii_ply(text: &str) -> Result<PointCloud> {
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("ply") {
        bail!("PLY file must start with 'ply'");
    }
    let mut ascii = false;
    let mut vertex_count = None;
    let mut face_count = 0usize;
    let mut section = "";
    let mut property_count = 0usize;
    let mut x_index = None;
    let mut y_index = None;
    let mut z_index = None;

    for line in lines.by_ref() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.first() == Some(&"end_header") {
            break;
        }
        match fields.as_slice() {
            ["format", format, ..] => ascii = *format == "ascii",
            ["element", "vertex", count] => {
                section = "vertex";
                vertex_count = Some(count.parse::<usize>().context("Invalid PLY vertex count")?);
                property_count = 0;
            }
            ["element", "face", count] => {
                section = "face";
                face_count = count.parse::<usize>().context("Invalid PLY face count")?;
            }
            ["property", _, name] if section == "vertex" => {
                match *name {
                    "x" => x_index = Some(property_count),
                    "y" => y_index = Some(property_count),
                    "z" => z_index = Some(property_count),
                    _ => {}
                }
                property_count += 1;
            }
            _ => {}
        }
    }
    if !ascii {
        bail!("Only ASCII PLY is supported for structural analysis");
    }
    let vertex_count = vertex_count.context("PLY header has no vertex element")?;
    if vertex_count > MAX_POINTS {
        bail!(
            "Point cloud exceeds the {} point analysis limit",
            MAX_POINTS
        );
    }
    let (x_index, y_index, z_index) = (
        x_index.context("PLY vertex property x is missing")?,
        y_index.context("PLY vertex property y is missing")?,
        z_index.context("PLY vertex property z is missing")?,
    );

    let mut cloud = PointCloud {
        points: Vec::with_capacity(vertex_count),
        faces: Vec::new(),
    };
    for line_number in 0..vertex_count {
        let line = lines
            .next()
            .context("PLY ended before all vertices were read")?;
        let values = line
            .split_whitespace()
            .map(str::parse::<f64>)
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("Invalid PLY vertex {}", line_number + 1))?;
        for value in [x_index, y_index, z_index] {
            if value >= values.len() || !values[value].is_finite() {
                bail!(
                    "Invalid finite X/Y/Z values for PLY vertex {}",
                    line_number + 1
                );
            }
        }
        cloud.points.push(Point {
            x: values[x_index],
            y: values[y_index],
            z: values[z_index],
        });
    }
    for _ in 0..face_count {
        let line = lines
            .next()
            .context("PLY ended before all faces were read")?;
        let indices = line
            .split_whitespace()
            .map(str::parse::<usize>)
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("Invalid PLY face")?;
        if indices.is_empty() || indices[0] + 1 > indices.len() {
            bail!("Invalid PLY face index list");
        }
        let vertices = &indices[1..];
        if vertices.iter().any(|index| *index >= vertex_count) {
            bail!("PLY face references a missing vertex");
        }
        for index in 1..vertices.len().saturating_sub(1) {
            cloud
                .faces
                .push([vertices[0], vertices[index], vertices[index + 1]]);
        }
    }
    if cloud.points.len() < 3 {
        bail!("At least three points are required");
    }
    Ok(cloud)
}

fn write_ascii_ply(output_path: &Path, cloud: &PointCloud) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(output_path)?;
    let mut writer = BufWriter::new(file);
    writeln!(
        writer,
        "ply\nformat ascii 1.0\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z",
        cloud.points.len()
    )?;
    if !cloud.faces.is_empty() {
        writeln!(
            writer,
            "element face {}\nproperty list uchar int vertex_indices",
            cloud.faces.len()
        )?;
    }
    writeln!(writer, "end_header")?;
    for point in &cloud.points {
        writeln!(writer, "{} {} {}", point.x, point.y, point.z)?;
    }
    for face in &cloud.faces {
        writeln!(writer, "3 {} {} {}", face[0], face[1], face[2])?;
    }
    writer.flush()?;
    Ok(())
}

fn extract_floorplan_layout(cloud: &PointCloud, cut_height: f64) -> Result<FloorplanLayout> {
    let (minimum, maximum) = point_bounds(&cloud.points);
    let span_y = (maximum[1] - minimum[1]).max(1e-4);
    let epsilon = span_y * 0.005;
    let candidate_offsets = [-0.03, -0.015, 0.0, 0.015, 0.03];
    let mut best = None;

    for offset in candidate_offsets {
        let sample_height =
            (cut_height + span_y * offset).clamp(minimum[1] + epsilon, maximum[1] - epsilon);
        let Some(polylines) = extract_floorplan_polylines(cloud, sample_height) else {
            continue;
        };
        let score = polyline_metric(&polylines[0]) + polylines.len() as f64;
        match best {
            Some((best_score, _)) if best_score >= score => {}
            _ => best = Some((score, polylines)),
        }
    }

    let mut polylines = best
        .map(|(_, polylines)| polylines)
        .context("No floorplan slice could be extracted at the requested cut height")?;
    polylines.sort_by(|left, right| {
        polyline_metric(right)
            .partial_cmp(&polyline_metric(left))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let raw_outer_boundary = closed_boundary(&polylines[0]);
    let outer_boundary = simplify_polyline(raw_outer_boundary.clone());
    let rooms = polylines
        .iter()
        .skip(1)
        .filter(|polyline| polyline.len() >= 3)
        .enumerate()
        .map(|(index, polyline)| FloorplanRoom {
            name: format!("Room {}", index + 1),
            boundary: simplify_polyline(closed_boundary(polyline)),
            raw_boundary: closed_boundary(polyline),
            wall_segments: Vec::new(),
            corners: Vec::new(),
        })
        .collect::<Vec<_>>();

    let mut layout = FloorplanLayout {
        outer_boundary,
        raw_outer_boundary,
        outer_walls: Vec::new(),
        outer_corners: Vec::new(),
        rooms,
        source: "model_slice_rust_best_fit_axes".to_string(),
    };
    layout.enrich_features();
    Ok(layout)
}

fn extract_floorplan_polylines(cloud: &PointCloud, cut_height: f64) -> Option<Vec<Vec<[f64; 2]>>> {
    let segments = slice_mesh_segments(cloud, cut_height);
    if segments.is_empty() {
        return None;
    }
    let polylines = chain_segments(&segments)
        .into_iter()
        .map(simplify_polyline)
        .filter(|polyline| polyline.len() >= 3)
        .collect::<Vec<_>>();
    (!polylines.is_empty()).then_some(polylines)
}

fn connect_neighbors(neighbors: &mut [Vec<usize>], left: usize, right: usize) {
    neighbors[left].push(right);
    neighbors[right].push(left);
}

fn relax_points(points: &mut [Point], neighbors: &[Vec<usize>], factor: f64) {
    let before = points.to_vec();
    for (index, point) in points.iter_mut().enumerate() {
        let adjacent = &neighbors[index];
        if adjacent.is_empty() {
            continue;
        }
        let mut average = Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        for neighbor in adjacent {
            average.x += before[*neighbor].x;
            average.y += before[*neighbor].y;
            average.z += before[*neighbor].z;
        }
        let count = adjacent.len() as f64;
        average.x /= count;
        average.y /= count;
        average.z /= count;
        point.x += (average.x - before[index].x) * factor;
        point.y += (average.y - before[index].y) * factor;
        point.z += (average.z - before[index].z) * factor;
    }
}

#[derive(Clone)]
struct SegmentConnection {
    idx: usize,
    other: (i64, i64),
    point: [f64; 2],
}

fn slice_mesh_segments(cloud: &PointCloud, cut_height: f64) -> Vec<[[f64; 2]; 2]> {
    let mut segments = Vec::new();
    for face in &cloud.faces {
        let points = [
            cloud.points[face[0]],
            cloud.points[face[1]],
            cloud.points[face[2]],
        ];
        let mut above = Vec::new();
        let mut below = Vec::new();
        for point in points {
            if point.y >= cut_height {
                above.push(point);
            } else {
                below.push(point);
            }
        }
        if above.is_empty() || below.is_empty() {
            continue;
        }
        let line = if above.len() == 1 && below.len() >= 2 {
            Some([
                intersect_plane_y(above[0], below[0], cut_height),
                intersect_plane_y(above[0], below[1], cut_height),
            ])
        } else if below.len() == 1 && above.len() >= 2 {
            Some([
                intersect_plane_y(below[0], above[0], cut_height),
                intersect_plane_y(below[0], above[1], cut_height),
            ])
        } else {
            None
        };
        if let Some([left, right]) = line {
            if distance_2d(left, right) > 1e-6 {
                segments.push([left, right]);
            }
        }
    }
    segments
}

fn intersect_plane_y(a: Point, b: Point, y: f64) -> [f64; 2] {
    let delta = b.y - a.y;
    let t = if delta.abs() <= 1e-12 {
        0.0
    } else {
        (y - a.y) / delta
    };
    [a.x + t * (b.x - a.x), a.z + t * (b.z - a.z)]
}

fn chain_segments(segments: &[[[f64; 2]; 2]]) -> Vec<Vec<[f64; 2]>> {
    let mut point_map: HashMap<(i64, i64), Vec<SegmentConnection>> = HashMap::new();
    for (idx, segment) in segments.iter().enumerate() {
        let h1 = hash_2d(segment[0]);
        let h2 = hash_2d(segment[1]);
        point_map.entry(h1).or_default().push(SegmentConnection {
            idx,
            other: h2,
            point: segment[1],
        });
        point_map.entry(h2).or_default().push(SegmentConnection {
            idx,
            other: h1,
            point: segment[0],
        });
    }

    let mut used = vec![false; segments.len()];
    let mut polylines = Vec::new();

    for (idx, segment) in segments.iter().enumerate() {
        if used[idx] {
            continue;
        }
        used[idx] = true;
        let mut polyline = vec![segment[0], segment[1]];
        extend_polyline(&mut polyline, &point_map, &mut used, true);
        extend_polyline(&mut polyline, &point_map, &mut used, false);
        polylines.push(polyline);
    }

    polylines
}

fn extend_polyline(
    polyline: &mut Vec<[f64; 2]>,
    point_map: &HashMap<(i64, i64), Vec<SegmentConnection>>,
    used: &mut [bool],
    forward: bool,
) {
    let mut cursor = if forward {
        hash_2d(*polyline.last().unwrap())
    } else {
        hash_2d(polyline[0])
    };

    loop {
        let Some(connections) = point_map.get(&cursor) else {
            break;
        };
        let Some(connection) = connections.iter().find(|connection| !used[connection.idx]) else {
            break;
        };
        used[connection.idx] = true;
        if forward {
            polyline.push(connection.point);
        } else {
            polyline.insert(0, connection.point);
        }
        cursor = connection.other;
    }
}

fn hash_2d(point: [f64; 2]) -> (i64, i64) {
    (
        (point[0] * 1000.0).round() as i64,
        (point[1] * 1000.0).round() as i64,
    )
}

fn simplify_polyline(mut polyline: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    polyline.dedup_by(|left, right| distance_2d(*left, *right) < 1e-6);
    if polyline.len() < 3 {
        return polyline;
    }
    let mut simplified = Vec::with_capacity(polyline.len());
    simplified.push(polyline[0]);
    for index in 1..polyline.len() - 1 {
        let prev = *simplified.last().unwrap();
        let current = polyline[index];
        let next = polyline[index + 1];
        if point_line_distance_2d(current, prev, next) > 0.01 {
            simplified.push(current);
        }
    }
    simplified.push(*polyline.last().unwrap());
    simplified
}

fn closed_boundary(polyline: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut boundary = polyline.to_vec();
    if boundary.len() >= 2 && distance_2d(boundary[0], *boundary.last().unwrap()) < 0.05 {
        boundary.pop();
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

fn distance_2d(left: [f64; 2], right: [f64; 2]) -> f64 {
    ((left[0] - right[0]).powi(2) + (left[1] - right[1]).powi(2)).sqrt()
}

fn polyline_metric(polyline: &[[f64; 2]]) -> f64 {
    let length = polyline
        .windows(2)
        .map(|segment| distance_2d(segment[0], segment[1]))
        .sum::<f64>();
    let area = polygon_area(polyline).abs();
    if area > 0.0 {
        area
    } else {
        length
    }
}

fn polygon_area(polyline: &[[f64; 2]]) -> f64 {
    if polyline.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for index in 0..polyline.len() {
        let current = polyline[index];
        let next = polyline[(index + 1) % polyline.len()];
        area += current[0] * next[1] - next[0] * current[1];
    }
    area * 0.5
}

fn point_bounds(points: &[Point]) -> ([f64; 3], [f64; 3]) {
    let mut minimum = [f64::INFINITY; 3];
    let mut maximum = [f64::NEG_INFINITY; 3];
    for point in points {
        for (axis, value) in [point.x, point.y, point.z].into_iter().enumerate() {
            minimum[axis] = minimum[axis].min(value);
            maximum[axis] = maximum[axis].max(value);
        }
    }
    (minimum, maximum)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary(name: &str) -> std::path::PathBuf {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("3dmk-{name}-{id}.ply"))
    }

    #[test]
    fn missing_input_is_rejected_before_pdal_runs() {
        let missing = std::env::temp_dir().join("3dmk-missing-point-cloud.ply");
        let output = std::env::temp_dir().join("3dmk-missing-output.ply");
        assert!(run_pdal_pipeline(&missing, &output).is_err());
    }

    #[test]
    fn rust_normal_estimation_discards_isolated_returns_without_failing() {
        let mut points = Vec::new();
        for x in 0..4 {
            for z in 0..4 {
                points.push(Point { x: x as f64, y: 0.0, z: z as f64 });
            }
        }
        points.push(Point { x: 100.0, y: 100.0, z: 100.0 });
        let (supported, normals) = estimate_local_normals(&points).expect("supported plane");
        assert!(supported.len() >= 12);
        assert_eq!(supported.len(), normals.len());
        assert!(normals.iter().all(|normal| normal.iter().all(|value| value.is_finite())));
    }

    #[test]
    fn ascii_ply_faces_are_preserved_when_flattened() {
        let input = temporary("input");
        let output = temporary("output");
        std::fs::write(&input, "ply\nformat ascii 1.0\nelement vertex 5\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 2 0\n1 2 0\n0 2 1\n1 2 1\n0.5 4 0.5\n4 0 1 3 2\n").unwrap();
        let result = flatten_file(&input, &output).unwrap();
        assert_eq!(result.plane.orientation, "horizontal");
        let corrected = read_ascii_point_cloud(&output).unwrap();
        assert_eq!(corrected.faces, vec![[0, 1, 3], [0, 3, 2]]);
        assert!(corrected
            .points
            .iter()
            .take(4)
            .all(|point| point.y.abs() < 1e-9));
        std::fs::remove_file(input).ok();
        std::fs::remove_file(output).ok();
    }

    #[test]
    fn structural_analysis_finds_a_horizontal_surface() {
        let input = temporary("analysis");
        let mut xyz = String::new();
        for x in 0..8 {
            for z in 0..8 {
                xyz.push_str(&format!("{x} 2 {z}\n"));
            }
        }
        xyz.push_str("4 5 4\n");
        std::fs::write(&input, xyz).unwrap();
        let analysis = analyze_file(&input).unwrap();
        assert!(analysis
            .planes
            .iter()
            .any(|plane| plane.orientation == "horizontal" && plane.inliers >= 60));
        std::fs::remove_file(input).ok();
    }

    #[test]
    fn default_scene_package_uses_loaded_mode() {
        let cloud = PointCloud {
            points: vec![
                Point {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
            ],
            faces: vec![[0, 1, 2]],
        };
        let package = default_scene_package(
            Path::new("sample.ply"),
            &cloud,
            Some(ScenePackageMetadata::default()),
        );
        assert_eq!(package.geometry_mode, GeometryMode::Mesh);
        assert_eq!(package.base_name, "sample");
    }

    #[test]
    fn canonical_scene_round_trips_mesh_geometry() {
        let cloud = PointCloud {
            points: vec![
                Point {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
            ],
            faces: vec![[0, 1, 2]],
        };
        let scene = point_cloud_to_canonical_scene(&cloud);
        assert!(scene.mesh);
        assert!(!scene.point_cloud);
        assert_eq!(scene.vertices.len(), 3);
        let round_trip = canonical_scene_to_point_cloud(&scene);
        assert_eq!(round_trip.points.len(), 3);
        assert_eq!(round_trip.faces, cloud.faces);
    }

    #[test]
    fn obj_loader_brings_in_triangulated_geometry() {
        let input = temporary("scene-obj").with_extension("obj");
        std::fs::write(&input, "o tri\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
        let (cloud, analysis) = load_scene_point_cloud(&input).unwrap();
        assert_eq!(cloud.points.len(), 3);
        assert_eq!(cloud.faces, vec![[0, 1, 2]]);
        assert!(analysis.is_none());
        std::fs::remove_file(input).ok();
    }

    #[test]
    fn ascii_ply_faces_are_preserved_when_smoothed() {
        let input = temporary("smooth-input");
        let output = temporary("smooth-output");
        std::fs::write(&input, "ply\nformat ascii 1.0\nelement vertex 4\nproperty float x\nproperty float y\nproperty float z\nelement face 2\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n1 0 1\n0 1 1\n3 0 1 2\n3 0 2 3\n").unwrap();
        let result = smooth_file(&input, &output, 2, 0.35).unwrap();
        let smoothed = read_ascii_point_cloud(&output).unwrap();
        assert_eq!(result.smoothed_points, 4);
        assert_eq!(smoothed.faces, vec![[0, 1, 2], [0, 2, 3]]);
        assert_eq!(smoothed.points.len(), 4);
        std::fs::remove_file(input).ok();
        std::fs::remove_file(output).ok();
    }

    #[test]
    fn floorplan_recognition_returns_outer_boundary() {
        let input = temporary("floorplan-input");
        std::fs::write(&input, "ply\nformat ascii 1.0\nelement vertex 8\nproperty float x\nproperty float y\nproperty float z\nelement face 12\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n4 0 0\n4 0 3\n0 0 3\n0 2 0\n4 2 0\n4 2 3\n0 2 3\n3 0 1 2\n3 0 2 3\n3 4 6 5\n3 4 7 6\n3 0 5 1\n3 0 4 5\n3 1 6 2\n3 1 5 6\n3 2 7 3\n3 2 6 7\n3 3 4 0\n3 3 7 4\n").unwrap();
        let result = recognize_floorplan_from_model(&input, 0.5, None).unwrap();
        assert!(result.layout.outer_boundary.len() >= 4);
        assert!(result.layout.raw_outer_boundary.len() >= 4);
        assert!(result.layout.outer_walls.len() >= 4);
        assert!(result.layout.outer_corners.len() >= 4);
        assert!(result
            .layout
            .outer_walls
            .iter()
            .all(|wall| wall.confidence.is_finite() && wall.confidence > 0.0));
        assert_eq!(result.layout.source, "model_slice_rust_best_fit_axes");
        std::fs::remove_file(input).ok();
    }
}
