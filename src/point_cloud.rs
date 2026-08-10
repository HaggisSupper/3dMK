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
use crate::scene::{
    build_analysis_context, build_analysis_context_with_settings, infer_geometry_mode, level_scene,
};
pub use crate::scene::{
    CloudAnalysis, CoordinateSystemMetadata, FlatSurfaceResult, FloatingMeshSettings, GeometryMode,
    MaterialBinding, Point, PointCloud, ScenePackageMetadata, TextureBinding,
};
use vwm_core::{CanonicalScene, GeometryAnalysis};
use vwm_geometry::{analyze_scene as analyze_vwm_scene, GeometryConfig};
use vwm_implicit::{
    sample_field_to_grid, GridSamplingConfig, ImplicitField, OrientedPointSet, PoissonConfig,
    PoissonReconstructor, SurfaceNetsConfig, SurfaceNetsExtractor,
};
use vwm_io::load_scene as load_vwm_scene;

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

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VwmReconstructionBackend {
    Auto,
    Pdal,
    Vwm,
}

impl Default for VwmReconstructionBackend {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VwmSurfaceExtraction {
    Poisson,
    SurfaceNets,
}

impl Default for VwmSurfaceExtraction {
    fn default() -> Self {
        Self::Poisson
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VwmReconstructionOptions {
    pub backend: VwmReconstructionBackend,
    pub extraction: VwmSurfaceExtraction,
    pub sample_limit: usize,
    pub screening: f64,
    pub density_estimation_depth: usize,
    pub max_depth: usize,
    pub max_relaxation_iterations: usize,
    pub surface_nets_resolution: u32,
}

impl Default for VwmReconstructionOptions {
    fn default() -> Self {
        Self {
            backend: VwmReconstructionBackend::Auto,
            extraction: VwmSurfaceExtraction::Poisson,
            sample_limit: 120_000,
            screening: 1.0,
            density_estimation_depth: 5,
            max_depth: 8,
            max_relaxation_iterations: 10,
            surface_nets_resolution: 64,
        }
    }
}

impl VwmReconstructionOptions {
    pub fn validate(self) -> Result<Self> {
        if !(4..=500_000).contains(&self.sample_limit) {
            bail!("VWM sample_limit must be between 4 and 500000");
        }
        if !(2..=12).contains(&self.max_depth)
            || self.density_estimation_depth > self.max_depth
            || self.max_relaxation_iterations == 0
            || self.max_relaxation_iterations > 100
            || !self.screening.is_finite()
            || !(0.0..=100.0).contains(&self.screening)
        {
            bail!("Invalid VWM Screened Poisson controls");
        }
        if !(16..=128).contains(&self.surface_nets_resolution) {
            bail!("VWM Surface Nets resolution must be between 16 and 128");
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VwmReconstructionReport {
    pub backend: &'static str,
    pub extraction: &'static str,
    pub source_points: usize,
    pub sampled_points: usize,
    pub output_vertices: usize,
    pub output_triangles: usize,
    pub options: VwmReconstructionOptions,
}

/// Pure-Rust fallback for machines without PDAL using the complete VWM controls.
pub fn run_implicit_pipeline_with_options(
    input_path: &Path,
    output_path: &Path,
    options: VwmReconstructionOptions,
) -> Result<VwmReconstructionReport> {
    let options = options.validate()?;
    let cloud = cleaned_reconstruction_cloud(input_path)?;
    if cloud.points.len() < 4 {
        bail!("Point-cloud reconstruction requires at least four points");
    }
    let source_points = cloud.points.len();
    let sample = sample_reconstruction_points(&cloud.points, options.sample_limit);
    let (sample, normals) = estimate_local_normals(&sample)?;
    let sampled_points = sample.len();
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
                screening: options.screening,
                max_depth: options.max_depth,
                density_estimation_depth: options.density_estimation_depth,
                max_relaxation_iterations: options.max_relaxation_iterations,
            },
        )
        .map_err(|error| anyhow!("Rust Poisson reconstruction failed: {error}"))?;
    let mesh = match options.extraction {
        VwmSurfaceExtraction::Poisson => field
            .reconstruct_mesh()
            .map_err(|error| anyhow!("Rust Poisson mesh extraction failed: {error}"))?,
        VwmSurfaceExtraction::SurfaceNets => {
            let resolution = options.surface_nets_resolution;
            let grid = sample_field_to_grid(
                &field,
                GridSamplingConfig {
                    bounds: field.bounds(),
                    dimensions: [resolution, resolution, resolution],
                },
            )
            .map_err(|error| anyhow!("VWM field sampling failed: {error}"))?;
            SurfaceNetsExtractor
                .extract(&grid, SurfaceNetsConfig::default())
                .map_err(|error| anyhow!("VWM Surface Nets extraction failed: {error}"))?
        }
    };
    if mesh.positions.is_empty() || mesh.triangles.is_empty() {
        bail!("Rust Poisson reconstruction produced no triangles");
    }
    let output_vertices = mesh.positions.len();
    let output_triangles = mesh.triangles.len();
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
    Ok(VwmReconstructionReport {
        backend: "vwm-implicit",
        extraction: match options.extraction {
            VwmSurfaceExtraction::Poisson => "screened_poisson",
            VwmSurfaceExtraction::SurfaceNets => "surface_nets",
        },
        source_points,
        sampled_points,
        output_vertices,
        output_triangles,
        options,
    })
}

pub fn run_implicit_pipeline(input_path: &Path, output_path: &Path) -> Result<()> {
    run_implicit_pipeline_with_options(input_path, output_path, VwmReconstructionOptions::default())
        .map(|_| ())
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VwmGeometryOptions {
    pub patch_normal_angle_threshold_degrees: f64,
    pub min_faces_per_patch: usize,
    pub plane_min_vertices: usize,
    pub parallel_angle_threshold_degrees: f64,
    pub perpendicular_angle_threshold_degrees: f64,
    pub adjacency_centroid_distance: f64,
    pub coplanar_offset_threshold: f64,
}

impl Default for VwmGeometryOptions {
    fn default() -> Self {
        let defaults = GeometryConfig::default();
        Self {
            patch_normal_angle_threshold_degrees: defaults.patch_normal_angle_threshold_degrees,
            min_faces_per_patch: defaults.min_faces_per_patch,
            plane_min_vertices: defaults.plane_min_vertices,
            parallel_angle_threshold_degrees: defaults.parallel_angle_threshold_degrees,
            perpendicular_angle_threshold_degrees: defaults.perpendicular_angle_threshold_degrees,
            adjacency_centroid_distance: defaults.adjacency_centroid_distance,
            coplanar_offset_threshold: defaults.coplanar_offset_threshold,
        }
    }
}

impl VwmGeometryOptions {
    fn geometry_config(self) -> Result<GeometryConfig> {
        let angles = [
            self.patch_normal_angle_threshold_degrees,
            self.parallel_angle_threshold_degrees,
            self.perpendicular_angle_threshold_degrees,
        ];
        if angles
            .iter()
            .any(|value| !value.is_finite() || !(0.1..=90.0).contains(value))
            || self.min_faces_per_patch == 0
            || self.plane_min_vertices < 3
            || !self.adjacency_centroid_distance.is_finite()
            || self.adjacency_centroid_distance <= 0.0
            || !self.coplanar_offset_threshold.is_finite()
            || self.coplanar_offset_threshold < 0.0
        {
            bail!("Invalid VWM geometry-analysis controls");
        }
        Ok(GeometryConfig {
            patch_normal_angle_threshold_degrees: self.patch_normal_angle_threshold_degrees,
            min_faces_per_patch: self.min_faces_per_patch,
            plane_min_vertices: self.plane_min_vertices,
            parallel_angle_threshold_degrees: self.parallel_angle_threshold_degrees,
            perpendicular_angle_threshold_degrees: self.perpendicular_angle_threshold_degrees,
            adjacency_centroid_distance: self.adjacency_centroid_distance,
            coplanar_offset_threshold: self.coplanar_offset_threshold,
        })
    }
}

pub fn analyze_vwm_geometry_file(
    input_path: &Path,
    options: VwmGeometryOptions,
) -> Result<vwm_core::GeometryAnalysis> {
    let scene = load_vwm_scene(input_path)?;
    analyze_vwm_scene(&scene, options.geometry_config()?)
}

/// Removes only clusters already classified as floating artifacts.
pub fn cleaned_reconstruction_cloud(input_path: &Path) -> Result<PointCloud> {
    let cloud = read_ascii_point_cloud(input_path)?;
    let analysis = build_analysis_context(&cloud, ScenePackageMetadata::default())?;
    let keep = analysis.reconstruction_keep_mask();
    let points = cloud
        .points
        .into_iter()
        .zip(keep)
        .filter_map(|(point, keep)| keep.then_some(point))
        .collect::<Vec<_>>();
    if points.len() < 4 {
        bail!("Artifact cleanup left fewer than four reconstruction points");
    }
    Ok(PointCloud {
        points,
        faces: Vec::new(),
    })
}

pub fn write_clean_reconstruction_input(input_path: &Path, output_path: &Path) -> Result<()> {
    let cloud = cleaned_reconstruction_cloud(input_path)?;
    write_ascii_ply(output_path, &cloud)
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
                                if index == *candidate_index {
                                    continue;
                                }
                                let distance = squared_distance(*point, points[*candidate_index]);
                                if distance < nearest[0].0 {
                                    nearest[2] = nearest[1];
                                    nearest[1] = nearest[0];
                                    nearest[0] = (distance, *candidate_index);
                                } else if distance < nearest[1].0 {
                                    nearest[2] = nearest[1];
                                    nearest[1] = (distance, *candidate_index);
                                } else if distance < nearest[2].0 {
                                    nearest[2] = (distance, *candidate_index);
                                }
                            }
                        }
                    }
                }
            }
            if nearest[2].0.is_finite() {
                break;
            }
        }
        // ponytail: isolated scan returns are discarded; upgrade to density-weighted confidence when cleanup needs to retain them.
        if !nearest[2].0.is_finite() {
            continue;
        }
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
        let outward = [
            point.x - centroid[0],
            point.y - centroid[1],
            point.z - centroid[2],
        ];
        if normal[0] * outward[0] + normal[1] * outward[1] + normal[2] * outward[2] < 0.0 {
            normal
                .iter_mut()
                .for_each(|component| *component = -*component);
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
    analyze_file_with_metadata_and_settings(input_path, package, FloatingMeshSettings::default())
}

pub fn analyze_file_with_metadata_and_settings(
    input_path: &Path,
    package: Option<ScenePackageMetadata>,
    settings: FloatingMeshSettings,
) -> Result<CloudAnalysis> {
    let (cloud, geometry_analysis) = load_scene_point_cloud(input_path)?;
    let package = default_scene_package(input_path, &cloud, package);
    let mut analysis = build_analysis_context_with_settings(&cloud, package, settings)?;
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

#[derive(Debug, Clone, Copy)]
enum PlyFaceProperty {
    Scalar,
    List { vertex_indices: bool },
}

fn triangulate_ply_face(vertices: &[usize], points: &[Point]) -> Result<Vec<[usize; 3]>> {
    const AREA_EPSILON: f64 = 1e-12;

    if vertices.len() < 3 {
        bail!("PLY faces must contain at least three vertices");
    }
    let unique = vertices
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    if unique.len() != vertices.len() {
        bail!("PLY face contains duplicate vertex indices");
    }

    let mut normal = [0.0f64; 3];
    for index in 0..vertices.len() {
        let current = &points[vertices[index]];
        let next = &points[vertices[(index + 1) % vertices.len()]];
        normal[0] += (current.y - next.y) * (current.z + next.z);
        normal[1] += (current.z - next.z) * (current.x + next.x);
        normal[2] += (current.x - next.x) * (current.y + next.y);
    }
    let dominant_axis = normal
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| {
            left.abs()
                .partial_cmp(&right.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(axis, _)| axis)
        .unwrap_or(2);
    if normal[dominant_axis].abs() <= AREA_EPSILON {
        bail!("PLY face is degenerate");
    }

    let projected = vertices
        .iter()
        .map(|vertex| {
            let point = &points[*vertex];
            match dominant_axis {
                0 => [point.y, point.z],
                1 => [point.x, point.z],
                _ => [point.x, point.y],
            }
        })
        .collect::<Vec<_>>();
    let mut orientation = 0.0f64;
    for index in 0..projected.len() {
        let previous = projected[(index + projected.len() - 1) % projected.len()];
        let current = projected[index];
        let next = projected[(index + 1) % projected.len()];
        let cross = (current[0] - previous[0]) * (next[1] - current[1])
            - (current[1] - previous[1]) * (next[0] - current[0]);
        if cross.abs() <= AREA_EPSILON {
            continue;
        }
        if orientation == 0.0 {
            orientation = cross.signum();
        } else if cross.signum() != orientation {
            bail!("Non-convex PLY polygon faces are not supported safely");
        }
    }
    if orientation == 0.0 {
        bail!("PLY face is degenerate");
    }

    Ok((1..vertices.len() - 1)
        .map(|index| [vertices[0], vertices[index], vertices[index + 1]])
        .collect())
}

fn parse_ascii_ply(text: &str) -> Result<PointCloud> {
    const MAX_FACE_VERTICES: usize = 4_096;
    const MAX_TRIANGLES: usize = MAX_POINTS * 8;

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
    let mut face_properties = Vec::<PlyFaceProperty>::new();
    let mut header_finished = false;

    for line in lines.by_ref() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.first() == Some(&"end_header") {
            header_finished = true;
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
                face_properties.clear();
            }
            ["element", _, _] => section = "unsupported",
            ["property", _, name] if section == "vertex" => {
                match *name {
                    "x" => x_index = Some(property_count),
                    "y" => y_index = Some(property_count),
                    "z" => z_index = Some(property_count),
                    _ => {}
                }
                property_count += 1;
            }
            ["property", "list", ..] if section == "vertex" => {
                bail!("PLY list properties on vertices are not supported")
            }
            ["property", "list", _, _, name] if section == "face" => {
                face_properties.push(PlyFaceProperty::List {
                    vertex_indices: matches!(*name, "vertex_indices" | "vertex_index"),
                });
            }
            ["property", _, _] if section == "face" => {
                face_properties.push(PlyFaceProperty::Scalar);
            }
            _ => {}
        }
    }
    if !header_finished {
        bail!("PLY header is missing end_header");
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
    if face_count > MAX_TRIANGLES {
        bail!("PLY exceeds the {} face analysis limit", MAX_TRIANGLES);
    }
    let vertex_index_lists = face_properties
        .iter()
        .filter(|property| {
            matches!(
                property,
                PlyFaceProperty::List {
                    vertex_indices: true
                }
            )
        })
        .count();
    if face_count > 0 && vertex_index_lists != 1 {
        bail!("PLY face element must define exactly one vertex_indices list");
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
    for face_number in 0..face_count {
        let line = lines
            .next()
            .context("PLY ended before all faces were read")?;
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let mut cursor = 0usize;
        let mut vertices = None::<Vec<usize>>;
        for property in &face_properties {
            match property {
                PlyFaceProperty::Scalar => {
                    let token = tokens.get(cursor).with_context(|| {
                        format!("PLY face {} is missing a scalar property", face_number + 1)
                    })?;
                    token.parse::<f64>().with_context(|| {
                        format!("Invalid scalar property on PLY face {}", face_number + 1)
                    })?;
                    cursor += 1;
                }
                PlyFaceProperty::List { vertex_indices } => {
                    let count = tokens
                        .get(cursor)
                        .with_context(|| {
                            format!("PLY face {} is missing a list count", face_number + 1)
                        })?
                        .parse::<usize>()
                        .with_context(|| {
                            format!("Invalid list count on PLY face {}", face_number + 1)
                        })?;
                    if count > MAX_FACE_VERTICES {
                        bail!(
                            "PLY face {} exceeds the {} vertex limit",
                            face_number + 1,
                            MAX_FACE_VERTICES
                        );
                    }
                    let list_start = cursor
                        .checked_add(1)
                        .context("PLY face list offset overflow")?;
                    let list_end = list_start
                        .checked_add(count)
                        .context("PLY face list length overflow")?;
                    let list = tokens.get(list_start..list_end).with_context(|| {
                        format!("PLY face {} list is shorter than declared", face_number + 1)
                    })?;
                    if *vertex_indices {
                        let parsed = list
                            .iter()
                            .map(|value| value.parse::<usize>())
                            .collect::<std::result::Result<Vec<_>, _>>()
                            .with_context(|| {
                                format!("Invalid vertex index on PLY face {}", face_number + 1)
                            })?;
                        vertices = Some(parsed);
                    } else {
                        for value in list {
                            value.parse::<f64>().with_context(|| {
                                format!("Invalid list property on PLY face {}", face_number + 1)
                            })?;
                        }
                    }
                    cursor = list_end;
                }
            }
        }
        if cursor != tokens.len() {
            bail!("PLY face {} has undeclared trailing data", face_number + 1);
        }
        let vertices = vertices.context("PLY face vertex_indices list is missing")?;
        if vertices.iter().any(|index| *index >= vertex_count) {
            bail!("PLY face references a missing vertex");
        }
        let triangles = triangulate_ply_face(&vertices, &cloud.points)?;
        if cloud.faces.len().saturating_add(triangles.len()) > MAX_TRIANGLES {
            bail!("PLY exceeds the {} triangle analysis limit", MAX_TRIANGLES);
        }
        cloud.faces.extend(triangles);
    }
    if cloud.points.len() < 3 {
        bail!("At least three points are required");
    }
    Ok(cloud)
}

#[cfg(test)]
mod adversarial_ply_parser_tests {
    use super::parse_ascii_ply;

    fn ply(vertices: &str, face_properties: &str, face: &str, vertex_count: usize) -> String {
        format!(
            "ply\nformat ascii 1.0\nelement vertex {vertex_count}\nproperty float x\nproperty float y\nproperty float z\nelement face 1\n{face_properties}\nend_header\n{vertices}{face}\n"
        )
    }

    #[test]
    fn face_scalar_properties_are_not_treated_as_indices() {
        let input = ply(
            "0 0 0\n1 0 0\n0 1 0\n",
            "property list uchar int vertex_indices\nproperty uchar material_id",
            "3 0 1 2 7",
            3,
        );
        let cloud = parse_ascii_ply(&input).expect("valid face with a trailing scalar property");
        assert_eq!(cloud.faces, vec![[0, 1, 2]]);
    }

    #[test]
    fn undeclared_face_tokens_are_rejected() {
        let input = ply(
            "0 0 0\n1 0 0\n0 1 0\n",
            "property list uchar int vertex_indices",
            "3 0 1 2 7",
            3,
        );
        assert!(parse_ascii_ply(&input)
            .expect_err("trailing data must not become a vertex index")
            .to_string()
            .contains("undeclared trailing data"));
    }

    #[test]
    fn convex_quads_are_triangulated_without_reading_other_properties() {
        let input = ply(
            "0 0 0\n1 0 0\n1 1 0\n0 1 0\n",
            "property uchar material_id\nproperty list uchar int vertex_indices",
            "4 4 0 1 2 3",
            4,
        );
        let cloud = parse_ascii_ply(&input).expect("convex quad should triangulate");
        assert_eq!(cloud.faces, vec![[0, 1, 2], [0, 2, 3]]);
    }

    #[test]
    fn non_convex_polygons_are_rejected_instead_of_fan_triangulated() {
        let input = ply(
            "0 0 0\n2 0 0\n1 0.5 0\n2 2 0\n0 2 0\n",
            "property list uchar int vertex_indices",
            "5 0 1 2 3 4",
            5,
        );
        assert!(parse_ascii_ply(&input)
            .expect_err("unsafe non-convex fan triangulation must be rejected")
            .to_string()
            .contains("Non-convex"));
    }
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
    let candidate_offsets = [
        -0.18, -0.15, -0.12, -0.09, -0.06, -0.03, 0.0, 0.03, 0.06, 0.09, 0.12, 0.15, 0.18,
    ];
    let mut candidates = Vec::new();

    for offset in candidate_offsets {
        let sample_height =
            (cut_height + span_y * offset).clamp(minimum[1] + epsilon, maximum[1] - epsilon);
        let Some(mut polylines) = extract_floorplan_polylines(cloud, sample_height) else {
            continue;
        };
        polylines.sort_by(|left, right| {
            polyline_metric(right)
                .partial_cmp(&polyline_metric(left))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.push(polylines);
    }

    let best_index = (0..candidates.len())
        .max_by(|left, right| {
            floorplan_candidate_score(*left, &candidates)
                .partial_cmp(&floorplan_candidate_score(*right, &candidates))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .context("No floorplan slice could be extracted at the requested cut height")?;
    let mut polylines = candidates[best_index].clone();
    let required_room_support = if candidates.len() >= 8 {
        3
    } else if candidates.len() >= 3 {
        2
    } else {
        1
    };
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
        .filter(|polyline| {
            candidates
                .iter()
                .filter(|candidate| {
                    candidate
                        .iter()
                        .skip(1)
                        .any(|other| floorplan_polyline_similarity(polyline, other) >= 0.62)
                })
                .count()
                >= required_room_support
        })
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
        source: "model_multislice_consensus_regularized".to_string(),
    };
    layout.enrich_features();
    Ok(layout)
}

fn floorplan_candidate_score(index: usize, candidates: &[Vec<Vec<[f64; 2]>>]) -> f64 {
    let outer = &candidates[index][0];
    let consensus = candidates
        .iter()
        .enumerate()
        .filter(|(other_index, _)| *other_index != index)
        .map(|(_, candidate)| floorplan_polyline_similarity(outer, &candidate[0]))
        .sum::<f64>();
    let closure = if outer.len() >= 3
        && distance_2d(outer[0], *outer.last().unwrap()) <= floorplan_polyline_scale(outer) * 0.025
    {
        1.0
    } else {
        0.0
    };
    consensus * 10.0
        + closure * 2.0
        + polyline_metric(outer).max(1e-6).ln_1p()
        + candidates[index].len().min(12) as f64 * 0.1
}

fn floorplan_polyline_similarity(left: &[[f64; 2]], right: &[[f64; 2]]) -> f64 {
    let left_area = polygon_area(left).abs();
    let right_area = polygon_area(right).abs();
    let area_ratio = left_area.min(right_area) / left_area.max(right_area).max(1e-6);
    let left_perimeter = floorplan_polyline_perimeter(left);
    let right_perimeter = floorplan_polyline_perimeter(right);
    let perimeter_ratio =
        left_perimeter.min(right_perimeter) / left_perimeter.max(right_perimeter).max(1e-6);
    let center_distance = distance_2d(
        floorplan_polyline_centroid(left),
        floorplan_polyline_centroid(right),
    );
    let center_score =
        (1.0 - center_distance / floorplan_polyline_scale(left).max(1e-6)).clamp(0.0, 1.0);
    area_ratio * 0.45 + perimeter_ratio * 0.25 + center_score * 0.30
}

fn floorplan_polyline_perimeter(polyline: &[[f64; 2]]) -> f64 {
    if polyline.len() < 2 {
        return 0.0;
    }
    (0..polyline.len())
        .map(|index| distance_2d(polyline[index], polyline[(index + 1) % polyline.len()]))
        .sum()
}

fn floorplan_polyline_centroid(polyline: &[[f64; 2]]) -> [f64; 2] {
    let count = polyline.len().max(1) as f64;
    let sum = polyline.iter().fold([0.0, 0.0], |mut sum, point| {
        sum[0] += point[0];
        sum[1] += point[1];
        sum
    });
    [sum[0] / count, sum[1] / count]
}

fn floorplan_polyline_scale(polyline: &[[f64; 2]]) -> f64 {
    let mut minimum = [f64::INFINITY; 2];
    let mut maximum = [f64::NEG_INFINITY; 2];
    for point in polyline {
        minimum[0] = minimum[0].min(point[0]);
        minimum[1] = minimum[1].min(point[1]);
        maximum[0] = maximum[0].max(point[0]);
        maximum[1] = maximum[1].max(point[1]);
    }
    distance_2d(minimum, maximum).max(1e-6)
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
                points.push(Point {
                    x: x as f64,
                    y: 0.0,
                    z: z as f64,
                });
            }
        }
        points.push(Point {
            x: 100.0,
            y: 100.0,
            z: 100.0,
        });
        let (supported, normals) = estimate_local_normals(&points).expect("supported plane");
        assert!(supported.len() >= 12);
        assert_eq!(supported.len(), normals.len());
        assert!(normals
            .iter()
            .all(|normal| normal.iter().all(|value| value.is_finite())));
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
        assert_eq!(
            result.layout.source,
            "model_multislice_consensus_regularized"
        );
        std::fs::remove_file(input).ok();
    }
}
