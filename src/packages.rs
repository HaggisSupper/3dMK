use crate::projects::{hex_digest, AssetRole, PackageAssetImport};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;
use zip::{CompressionMethod, ZipArchive};

pub const ARCHIVE_INVENTORY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug)]
pub struct ArchiveLimits {
    pub max_archive_bytes: u64,
    pub max_entries: usize,
    pub max_total_expanded_bytes: u64,
    pub max_entry_expanded_bytes: u64,
    pub max_compression_ratio: u64,
    pub max_path_bytes: usize,
    pub max_path_depth: usize,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_archive_bytes: 512 * 1024 * 1024,
            max_entries: 512,
            max_total_expanded_bytes: 1024 * 1024 * 1024,
            max_entry_expanded_bytes: 512 * 1024 * 1024,
            max_compression_ratio: 200,
            max_path_bytes: 512,
            max_path_depth: 16,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    #[error("archive file could not be read")]
    Io(#[from] std::io::Error),
    #[error("archive is not a valid supported ZIP")]
    InvalidArchive,
    #[error("archive exceeds the compressed byte limit")]
    ArchiveTooLarge,
    #[error("archive contains too many entries")]
    TooManyEntries,
    #[error("archive entry path is unsafe")]
    UnsafePath,
    #[error("archive contains duplicate normalized entry paths")]
    DuplicatePath,
    #[error("archive contains a link or special filesystem entry")]
    UnsupportedLink,
    #[error("archive contains an encrypted entry")]
    EncryptedEntry,
    #[error("archive uses an unsupported compression method")]
    UnsupportedCompression,
    #[error("archive entry exceeds the expanded byte limit")]
    EntryTooLarge,
    #[error("archive exceeds the aggregate expanded byte limit")]
    ExpandedArchiveTooLarge,
    #[error("archive entry exceeds the compression-ratio limit")]
    CompressionRatioTooHigh,
    #[error("archive entry size does not match its central-directory record")]
    EntrySizeMismatch,
    #[error("iPhone capture metadata is invalid")]
    InvalidCaptureMetadata,
    #[error("package does not contain a supported primary model")]
    MissingPrimaryModel,
    #[error("package contains multiple equally ranked models and requires a primary-model choice")]
    PrimaryModelChoiceRequired,
    #[error("selected primary model is not a supported model in the package")]
    InvalidPrimaryModel,
}

impl PackageError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io(_) | Self::InvalidArchive | Self::EntrySizeMismatch => "invalid_archive",
            Self::ArchiveTooLarge => "archive_too_large",
            Self::TooManyEntries => "archive_entry_limit_exceeded",
            Self::UnsafePath => "unsafe_archive_path",
            Self::DuplicatePath => "duplicate_archive_path",
            Self::UnsupportedLink => "unsupported_archive_link",
            Self::EncryptedEntry => "encrypted_archive_entry",
            Self::UnsupportedCompression => "unsupported_archive_compression",
            Self::EntryTooLarge => "archive_entry_too_large",
            Self::ExpandedArchiveTooLarge => "archive_expanded_limit_exceeded",
            Self::CompressionRatioTooHigh => "archive_compression_ratio_exceeded",
            Self::InvalidCaptureMetadata => "invalid_capture_metadata",
            Self::MissingPrimaryModel => "missing_primary_model",
            Self::PrimaryModelChoiceRequired => "primary_model_choice_required",
            Self::InvalidPrimaryModel => "invalid_primary_model",
        }
    }
}

pub type Result<T> = std::result::Result<T, PackageError>;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ArchiveEntry {
    pub path: String,
    pub directory: bool,
    pub compressed_bytes: u64,
    pub expanded_bytes: u64,
    pub compression: String,
    pub media_type: String,
    pub sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AdapterCandidate {
    pub id: String,
    pub confidence: u8,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ModelCandidate {
    pub path: String,
    pub format: String,
    pub score: u16,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PrimaryModelResolution {
    pub selected_path: Option<String>,
    pub requires_user_choice: bool,
    pub reason: String,
    pub candidates: Vec<ModelCandidate>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ArchiveInventory {
    pub schema_version: u32,
    pub archive_bytes: u64,
    pub entry_count: usize,
    pub file_count: usize,
    pub directory_count: usize,
    pub total_expanded_bytes: u64,
    pub entries: Vec<ArchiveEntry>,
    pub adapter_candidates: Vec<AdapterCandidate>,
    pub primary_model: PrimaryModelResolution,
    pub iphone_capture: Option<IphoneCaptureReport>,
}

#[derive(Debug)]
pub struct PreparedPackage {
    pub inventory: ArchiveInventory,
    pub selected_model_path: String,
    pub assets: Vec<PackageAssetImport>,
    extraction_dir: PathBuf,
}

impl Drop for PreparedPackage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.extraction_dir);
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CaptureWarning {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PointDescriptorSummary {
    pub point_count: u64,
    pub block_count: u64,
    pub has_colors: bool,
    pub has_normals: bool,
    pub format: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MissingImageRecord {
    pub index: u64,
    pub image_path: String,
    pub missing_image: bool,
    pub missing_camera_json: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CameraConvention {
    pub transform: String,
    pub local_axes: String,
    pub image_origin: String,
    pub intrinsic_units: String,
    pub projection: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CanonicalCamera {
    pub id: String,
    pub index: u64,
    pub image_path: String,
    pub camera_json_path: String,
    pub dimensions: [u32; 2],
    pub focal_pixels: [f64; 2],
    pub principal_point_pixels: [f64; 2],
    pub world_from_camera: [[f64; 4]; 4],
    pub image_present: bool,
    pub camera_json_present: bool,
    pub calibration_valid: bool,
}

impl CanonicalCamera {
    pub fn project_world(&self, point: [f64; 3]) -> Option<[f64; 2]> {
        self.project_world_with_depth(point)
            .map(|(pixel, _depth)| pixel)
    }

    pub fn project_world_with_depth(&self, point: [f64; 3]) -> Option<([f64; 2], f64)> {
        if !self.calibration_valid || point.iter().any(|value| !value.is_finite()) {
            return None;
        }
        let camera_center = [
            self.world_from_camera[0][3],
            self.world_from_camera[1][3],
            self.world_from_camera[2][3],
        ];
        let delta = [
            point[0] - camera_center[0],
            point[1] - camera_center[1],
            point[2] - camera_center[2],
        ];
        let camera_x = dot_column(&self.world_from_camera, 0, delta);
        let camera_y = dot_column(&self.world_from_camera, 1, delta);
        let depth = -dot_column(&self.world_from_camera, 2, delta);
        if depth <= 1e-9 {
            return None;
        }
        Some((
            [
                self.focal_pixels[0] * camera_x / depth + self.principal_point_pixels[0],
                self.focal_pixels[1] * -camera_y / depth + self.principal_point_pixels[1],
            ],
            depth,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct IphoneCaptureReport {
    pub schema_version: u32,
    pub capture_id: String,
    pub name: String,
    pub primary_model_path: String,
    pub expected_image_records: usize,
    pub descriptor_records: usize,
    pub observed_images: usize,
    pub observed_camera_json: usize,
    pub valid_image_pairs: usize,
    pub missing_records: Vec<MissingImageRecord>,
    pub raw_points: Option<PointDescriptorSummary>,
    pub cloud_points: Option<PointDescriptorSummary>,
    pub camera_convention: CameraConvention,
    pub calibration_valid: bool,
    pub cameras: Vec<CanonicalCamera>,
    pub warnings: Vec<CaptureWarning>,
}

const MAX_CALIBRATED_CAMERAS: usize = 4096;
const RIGID_TRANSFORM_TOLERANCE: f64 = 5.0e-3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CameraBindingError {
    InvalidSourcePath,
    CameraIdNotFound,
    DuplicateCameraId,
    SourcePathMismatch,
    CameraUnavailable,
}

impl CameraBindingError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::InvalidSourcePath => "invalid_source_path",
            Self::CameraIdNotFound => "camera_id_not_found",
            Self::DuplicateCameraId => "duplicate_camera_id",
            Self::SourcePathMismatch => "camera_source_path_mismatch",
            Self::CameraUnavailable => "camera_pair_unavailable",
        }
    }
}

pub fn normalized_package_path(value: &str) -> Option<String> {
    normalize_entry_path(value.as_bytes(), ArchiveLimits::default())
        .ok()
        .map(|path| path.to_ascii_lowercase())
}

pub fn camera_calibration_is_valid(camera: &CanonicalCamera) -> bool {
    let [width, height] = camera.dimensions;
    if width == 0
        || height == 0
        || width > 100_000
        || height > 100_000
        || u64::from(width) * u64::from(height) > 100_000_000
        || !valid_camera(
            camera.dimensions,
            camera.focal_pixels,
            camera.principal_point_pixels,
            &camera.world_from_camera,
        )
    {
        return false;
    }

    let rotation = [
        [
            camera.world_from_camera[0][0],
            camera.world_from_camera[1][0],
            camera.world_from_camera[2][0],
        ],
        [
            camera.world_from_camera[0][1],
            camera.world_from_camera[1][1],
            camera.world_from_camera[2][1],
        ],
        [
            camera.world_from_camera[0][2],
            camera.world_from_camera[1][2],
            camera.world_from_camera[2][2],
        ],
    ];
    let dot = |left: [f64; 3], right: [f64; 3]| {
        left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
    };
    let columns_are_orthonormal = rotation
        .iter()
        .all(|column| (dot(*column, *column) - 1.0).abs() <= RIGID_TRANSFORM_TOLERANCE)
        && dot(rotation[0], rotation[1]).abs() <= RIGID_TRANSFORM_TOLERANCE
        && dot(rotation[0], rotation[2]).abs() <= RIGID_TRANSFORM_TOLERANCE
        && dot(rotation[1], rotation[2]).abs() <= RIGID_TRANSFORM_TOLERANCE;
    let determinant = rotation[0][0]
        * (rotation[1][1] * rotation[2][2] - rotation[1][2] * rotation[2][1])
        - rotation[1][0]
            * (rotation[0][1] * rotation[2][2] - rotation[0][2] * rotation[2][1])
        + rotation[2][0]
            * (rotation[0][1] * rotation[1][2] - rotation[0][2] * rotation[1][1]);
    let affine_row = camera.world_from_camera[3];

    columns_are_orthonormal
        && (determinant - 1.0).abs() <= RIGID_TRANSFORM_TOLERANCE * 2.0
        && affine_row[0].abs() <= RIGID_TRANSFORM_TOLERANCE
        && affine_row[1].abs() <= RIGID_TRANSFORM_TOLERANCE
        && affine_row[2].abs() <= RIGID_TRANSFORM_TOLERANCE
        && (affine_row[3] - 1.0).abs() <= RIGID_TRANSFORM_TOLERANCE
}

pub fn validate_iphone_capture_report(
    report: &IphoneCaptureReport,
) -> std::result::Result<(), &'static str> {
    if report.schema_version != 1 {
        return Err("unsupported camera report schema");
    }
    if report.capture_id.trim().is_empty() {
        return Err("camera report capture_id is required");
    }
    if report.cameras.is_empty() || report.cameras.len() > MAX_CALIBRATED_CAMERAS {
        return Err("camera report has an invalid camera count");
    }
    if report.descriptor_records != report.cameras.len() {
        return Err("camera report descriptor count is inconsistent");
    }
    if normalized_package_path(&report.primary_model_path).is_none() {
        return Err("camera report primary model path is unsafe");
    }
    if [
        report.camera_convention.transform.as_str(),
        report.camera_convention.local_axes.as_str(),
        report.camera_convention.image_origin.as_str(),
        report.camera_convention.intrinsic_units.as_str(),
        report.camera_convention.projection.as_str(),
    ]
    .iter()
    .any(|value| value.trim().is_empty())
    {
        return Err("camera convention is incomplete");
    }

    let mut camera_ids = HashSet::with_capacity(report.cameras.len());
    let mut camera_indices = HashSet::with_capacity(report.cameras.len());
    let mut image_paths = HashSet::with_capacity(report.cameras.len());
    let mut camera_json_paths = HashSet::with_capacity(report.cameras.len());
    for camera in &report.cameras {
        if camera.id.trim().is_empty()
            || !camera_ids.insert(camera.id.to_ascii_lowercase())
            || !camera_indices.insert(camera.index)
        {
            return Err("camera report contains duplicate or empty camera identity");
        }
        let image_path = normalized_package_path(&camera.image_path)
            .ok_or("camera report contains an unsafe image path")?;
        let camera_json_path = normalized_package_path(&camera.camera_json_path)
            .ok_or("camera report contains an unsafe camera JSON path")?;
        if !image_paths.insert(image_path.clone()) || !camera_json_paths.insert(camera_json_path.clone()) {
            return Err("camera report contains duplicate camera paths");
        }
        let expected_camera_json = image_path
            .rsplit_once('.')
            .map_or_else(|| format!("{image_path}.json"), |(stem, _)| format!("{stem}.json"));
        if camera_json_path != expected_camera_json {
            return Err("camera JSON path does not correspond to its image path");
        }
        if !camera.calibration_valid || !camera_calibration_is_valid(camera) {
            return Err("camera report contains invalid intrinsics or a non-rigid transform");
        }
    }

    let valid_image_pairs = report
        .cameras
        .iter()
        .filter(|camera| camera.image_present && camera.camera_json_present)
        .count();
    if !report.calibration_valid
        || valid_image_pairs == 0
        || report.valid_image_pairs != valid_image_pairs
        || report.observed_images < valid_image_pairs
        || report.observed_camera_json < valid_image_pairs
    {
        return Err("camera report validity counts are inconsistent");
    }
    Ok(())
}

pub fn camera_for_explicit_binding<'a>(
    cameras: &'a [CanonicalCamera],
    camera_id: &str,
    source_path: &str,
) -> std::result::Result<&'a CanonicalCamera, CameraBindingError> {
    let normalized_source =
        normalized_package_path(source_path).ok_or(CameraBindingError::InvalidSourcePath)?;
    let mut id_matches = cameras
        .iter()
        .filter(|camera| camera.id == camera_id);
    let camera = id_matches
        .next()
        .ok_or(CameraBindingError::CameraIdNotFound)?;
    if id_matches.next().is_some() {
        return Err(CameraBindingError::DuplicateCameraId);
    }
    if normalized_package_path(&camera.image_path).as_deref() != Some(&normalized_source) {
        return Err(CameraBindingError::SourcePathMismatch);
    }
    if !camera.image_present
        || !camera.camera_json_present
        || !camera.calibration_valid
        || !camera_calibration_is_valid(camera)
    {
        return Err(CameraBindingError::CameraUnavailable);
    }
    Ok(camera)
}

#[cfg(test)]
mod calibrated_camera_contract_tests {
    use super::*;

    fn camera(id: &str, index: u64, image_path: &str) -> CanonicalCamera {
        let camera_json_path = image_path
            .rsplit_once('.')
            .map_or_else(|| format!("{image_path}.json"), |(stem, _)| format!("{stem}.json"));
        CanonicalCamera {
            id: id.to_owned(),
            index,
            image_path: image_path.to_owned(),
            camera_json_path,
            dimensions: [128, 128],
            focal_pixels: [96.0, 96.0],
            principal_point_pixels: [64.0, 64.0],
            world_from_camera: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            image_present: true,
            camera_json_present: true,
            calibration_valid: true,
        }
    }

    fn report(cameras: Vec<CanonicalCamera>) -> IphoneCaptureReport {
        IphoneCaptureReport {
            schema_version: 1,
            capture_id: "capture-1".to_owned(),
            name: "Capture".to_owned(),
            primary_model_path: "Refined-Mesh-1.glb".to_owned(),
            expected_image_records: cameras.len(),
            descriptor_records: cameras.len(),
            observed_images: cameras.len(),
            observed_camera_json: cameras.len(),
            valid_image_pairs: cameras.len(),
            missing_records: vec![],
            raw_points: None,
            cloud_points: None,
            camera_convention: CameraConvention {
                transform: "world_from_camera".to_owned(),
                local_axes: "+x right, +y up, -z forward".to_owned(),
                image_origin: "top_left".to_owned(),
                intrinsic_units: "pixels".to_owned(),
                projection: "pinhole".to_owned(),
            },
            calibration_valid: true,
            cameras,
            warnings: vec![],
        }
    }

    #[test]
    fn calibrated_camera_report_rejects_empty_duplicates_and_non_rigid_transforms() {
        let valid = report(vec![camera("camera-a", 1, "RawImages/a.jpg")]);
        assert!(validate_iphone_capture_report(&valid).is_ok());

        let mut empty = report(vec![]);
        empty.calibration_valid = false;
        assert!(validate_iphone_capture_report(&empty).is_err());

        let first = camera("camera-a", 1, "RawImages/a.jpg");
        let mut duplicate_id = camera("camera-a", 2, "RawImages/b.jpg");
        let duplicate_report = report(vec![first.clone(), duplicate_id.clone()]);
        assert!(validate_iphone_capture_report(&duplicate_report).is_err());

        duplicate_id.id = "camera-b".to_owned();
        duplicate_id.index = 1;
        assert!(validate_iphone_capture_report(&report(vec![first.clone(), duplicate_id.clone()])).is_err());

        duplicate_id.index = 2;
        duplicate_id.image_path = first.image_path.clone();
        duplicate_id.camera_json_path = first.camera_json_path.clone();
        assert!(validate_iphone_capture_report(&report(vec![first.clone(), duplicate_id])).is_err());

        let mut non_rigid = first;
        non_rigid.world_from_camera[0][0] = 1.25;
        assert!(validate_iphone_capture_report(&report(vec![non_rigid])).is_err());
    }

    #[test]
    fn calibrated_camera_binding_requires_both_id_and_normalized_exact_source_path() {
        let cameras = vec![
            camera("camera-a", 1, "RawImages/room/a.jpg"),
            camera("camera-b", 2, "RawImages/other/a.jpg"),
        ];
        let matched = camera_for_explicit_binding(
            &cameras,
            "camera-a",
            r"RawImages\room\a.jpg",
        )
        .unwrap();
        assert_eq!(matched.index, 1);
        assert_eq!(
            camera_for_explicit_binding(&cameras, "camera-b", "RawImages/room/a.jpg"),
            Err(CameraBindingError::SourcePathMismatch)
        );
        assert_eq!(
            camera_for_explicit_binding(&cameras, "camera-a", "../RawImages/room/a.jpg"),
            Err(CameraBindingError::InvalidSourcePath)
        );
    }
}

#[derive(Deserialize)]
struct SessionDescriptor {
    name: String,
    uuid: String,
    #[serde(rename = "textureImageDescriptor")]
    texture_images: TextureImageDescriptor,
    #[serde(rename = "rawPointsDescriptor")]
    raw_points: Option<RawPointDescriptor>,
    #[serde(rename = "cloudDescriptor")]
    cloud: Option<CloudDescriptor>,
}

#[derive(Deserialize)]
struct TextureImageDescriptor {
    #[serde(rename = "directoryName")]
    directory_name: String,
    count: usize,
}

#[derive(Deserialize)]
struct RawPointDescriptor {
    #[serde(rename = "pointCount")]
    point_count: u64,
    #[serde(rename = "blockCount")]
    block_count: u64,
    #[serde(rename = "hasColors")]
    has_colors: bool,
    #[serde(rename = "hasNormals")]
    has_normals: bool,
    format: String,
}

#[derive(Deserialize)]
struct CloudDescriptor {
    #[serde(rename = "pointDescriptor")]
    point_descriptor: RawPointDescriptor,
}

#[derive(Deserialize)]
struct RawImageDescriptor {
    dimensions: [u32; 2],
    file_path: String,
    id: String,
    index: u64,
    pose: RawCameraPose,
}

#[derive(Deserialize)]
struct RawCameraPose {
    focal_length: [f64; 2],
    principal_point: [f64; 2],
    rotation_matrix: [f64; 9],
    translation: [f64; 3],
}

pub fn inspect_archive(path: &Path, limits: ArchiveLimits) -> Result<ArchiveInventory> {
    let archive_bytes = std::fs::metadata(path)?.len();
    if archive_bytes > limits.max_archive_bytes {
        return Err(PackageError::ArchiveTooLarge);
    }
    inspect_reader(File::open(path)?, archive_bytes, limits)
}

pub fn prepare_archive(
    archive_path: &Path,
    extraction_root: &Path,
    selected_model: Option<&str>,
    limits: ArchiveLimits,
) -> Result<PreparedPackage> {
    let inventory = inspect_archive(archive_path, limits)?;
    let selected_model_path = resolve_selected_model(&inventory, selected_model)?;
    fs::create_dir_all(extraction_root)?;
    let extraction_dir = extraction_root.join(format!("package-{}", Uuid::new_v4()));
    fs::create_dir(&extraction_dir)?;

    let result = (|| {
        extract_verified_entries(archive_path, &extraction_dir, &inventory, limits)?;
        let mut assets = package_assets(&extraction_dir, &inventory, &selected_model_path)?;
        add_derived_reports(&extraction_dir, &inventory, &mut assets)?;
        Ok(PreparedPackage {
            inventory,
            selected_model_path,
            assets,
            extraction_dir: extraction_dir.clone(),
        })
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&extraction_dir);
    }
    result
}

fn resolve_selected_model(inventory: &ArchiveInventory, requested: Option<&str>) -> Result<String> {
    if let Some(requested) = requested.map(str::trim).filter(|value| !value.is_empty()) {
        return inventory
            .primary_model
            .candidates
            .iter()
            .find(|candidate| candidate.path.eq_ignore_ascii_case(requested))
            .map(|candidate| candidate.path.clone())
            .ok_or(PackageError::InvalidPrimaryModel);
    }
    if inventory.primary_model.requires_user_choice {
        return Err(PackageError::PrimaryModelChoiceRequired);
    }
    inventory
        .primary_model
        .selected_path
        .clone()
        .ok_or(PackageError::MissingPrimaryModel)
}

fn extract_verified_entries(
    archive_path: &Path,
    extraction_dir: &Path,
    inventory: &ArchiveInventory,
    limits: ArchiveLimits,
) -> Result<()> {
    let expected = inventory
        .entries
        .iter()
        .map(|entry| (entry.path.to_ascii_lowercase(), entry))
        .collect::<HashMap<_, _>>();
    let mut observed = HashSet::with_capacity(expected.len());
    let mut archive =
        ZipArchive::new(File::open(archive_path)?).map_err(|_| PackageError::InvalidArchive)?;
    if archive.len() != inventory.entry_count || archive.len() > limits.max_entries {
        return Err(PackageError::InvalidArchive);
    }

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|_| PackageError::InvalidArchive)?;
        let path = normalize_entry_path(entry.name_raw(), limits)?;
        let key = path.to_ascii_lowercase();
        let expected_entry = expected.get(&key).ok_or(PackageError::InvalidArchive)?;
        if !observed.insert(key) {
            return Err(PackageError::DuplicatePath);
        }
        if entry.encrypted()
            || entry.is_symlink()
            || is_special_unix_entry(entry.unix_mode())
            || !matches!(
                entry.compression(),
                CompressionMethod::Stored | CompressionMethod::Deflated
            )
        {
            return Err(PackageError::InvalidArchive);
        }
        if entry.size() != expected_entry.expanded_bytes
            || entry.compressed_size() != expected_entry.compressed_bytes
            || entry.is_dir() != expected_entry.directory
        {
            return Err(PackageError::EntrySizeMismatch);
        }

        let output_path = extracted_path(extraction_dir, &path);
        if expected_entry.directory {
            fs::create_dir_all(output_path)?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&output_path)?;
        let mut digest = Sha256::new();
        let mut observed_bytes = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = entry
                .read(&mut buffer)
                .map_err(|_| PackageError::InvalidArchive)?;
            if read == 0 {
                break;
            }
            observed_bytes = observed_bytes
                .checked_add(read as u64)
                .ok_or(PackageError::EntryTooLarge)?;
            if observed_bytes > expected_entry.expanded_bytes
                || observed_bytes > limits.max_entry_expanded_bytes
            {
                return Err(PackageError::EntrySizeMismatch);
            }
            digest.update(&buffer[..read]);
            output.write_all(&buffer[..read])?;
        }
        output.sync_all()?;
        if observed_bytes != expected_entry.expanded_bytes
            || expected_entry.sha256.as_deref() != Some(hex_digest(&digest.finalize()).as_str())
        {
            return Err(PackageError::EntrySizeMismatch);
        }
    }
    if observed.len() != expected.len() {
        return Err(PackageError::InvalidArchive);
    }
    Ok(())
}

fn extracted_path(root: &Path, package_path: &str) -> PathBuf {
    package_path
        .split('/')
        .fold(root.to_path_buf(), |path, component| path.join(component))
}

fn package_assets(
    extraction_dir: &Path,
    inventory: &ArchiveInventory,
    selected_model_path: &str,
) -> Result<Vec<PackageAssetImport>> {
    let iphone_images = inventory
        .iphone_capture
        .iter()
        .flat_map(|capture| capture.cameras.iter())
        .map(|camera| camera.image_path.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let iphone_camera_json = inventory
        .iphone_capture
        .iter()
        .flat_map(|capture| capture.cameras.iter())
        .map(|camera| camera.camera_json_path.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut assets = Vec::<PackageAssetImport>::new();
    let mut by_digest = HashMap::<String, usize>::new();

    for entry in inventory.entries.iter().filter(|entry| !entry.directory) {
        let digest = entry.sha256.clone().ok_or(PackageError::InvalidArchive)?;
        let role = package_asset_role(
            entry,
            selected_model_path,
            &iphone_images,
            &iphone_camera_json,
        );
        if let Some(index) = by_digest.get(&digest).copied() {
            assets[index].package_paths.push(entry.path.clone());
            if asset_role_priority(role) > asset_role_priority(assets[index].role) {
                assets[index].source_path = extracted_path(extraction_dir, &entry.path);
                assets[index].original_name = package_basename(&entry.path);
                assets[index].media_type = entry.media_type.clone();
                assets[index].role = role;
            }
            continue;
        }
        by_digest.insert(digest.clone(), assets.len());
        assets.push(PackageAssetImport {
            source_path: extracted_path(extraction_dir, &entry.path),
            package_paths: vec![entry.path.clone()],
            original_name: package_basename(&entry.path),
            media_type: entry.media_type.clone(),
            role,
            source: true,
            expected_sha256: Some(digest),
            expected_bytes: Some(entry.expanded_bytes),
            metadata: serde_json::json!({
                "package_entry": true,
                "compression": entry.compression,
                "compressed_bytes": entry.compressed_bytes,
            }),
        });
    }
    Ok(assets)
}

fn package_asset_role(
    entry: &ArchiveEntry,
    selected_model_path: &str,
    iphone_images: &HashSet<String>,
    iphone_camera_json: &HashSet<String>,
) -> AssetRole {
    if entry.path.eq_ignore_ascii_case(selected_model_path) {
        return AssetRole::SourceModel;
    }
    let lower = entry.path.to_ascii_lowercase();
    if iphone_images.contains(&lower) {
        return AssetRole::Photo;
    }
    if iphone_camera_json.contains(&lower)
        || matches!(lower.as_str(), "session.json" | "rawimages_blocks.json")
    {
        return AssetRole::Calibration;
    }
    if matches!(entry.media_type.as_str(), "image/png" | "image/jpeg") {
        return AssetRole::Texture;
    }
    AssetRole::Other
}

fn asset_role_priority(role: AssetRole) -> u8 {
    match role {
        AssetRole::SourceModel => 5,
        AssetRole::Photo => 4,
        AssetRole::Texture => 3,
        AssetRole::Calibration => 2,
        _ => 1,
    }
}

fn package_basename(path: &str) -> String {
    path.rsplit('/').next().unwrap_or("asset").to_owned()
}

fn add_derived_reports(
    extraction_dir: &Path,
    inventory: &ArchiveInventory,
    assets: &mut Vec<PackageAssetImport>,
) -> Result<()> {
    let used_paths = inventory
        .entries
        .iter()
        .map(|entry| entry.path.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let inventory_path = unique_report_path(&used_paths, "package-inventory.json");
    let inventory_file = extracted_path(extraction_dir, &inventory_path);
    if let Some(parent) = inventory_file.parent() {
        fs::create_dir_all(parent)?;
    }
    let inventory_bytes =
        serde_json::to_vec_pretty(inventory).map_err(|_| PackageError::InvalidArchive)?;
    fs::write(&inventory_file, &inventory_bytes)?;
    assets.push(PackageAssetImport {
        source_path: inventory_file,
        package_paths: vec![inventory_path],
        original_name: "package-inventory.json".to_owned(),
        media_type: "application/json".to_owned(),
        role: AssetRole::Report,
        source: false,
        expected_sha256: Some(hex_digest(&Sha256::digest(&inventory_bytes))),
        expected_bytes: Some(inventory_bytes.len() as u64),
        metadata: serde_json::json!({ "derived_report": "package_inventory" }),
    });

    if let Some(capture) = &inventory.iphone_capture {
        let mut reserved = used_paths;
        reserved.extend(
            assets
                .iter()
                .flat_map(|asset| asset.package_paths.iter())
                .map(|path| path.to_ascii_lowercase()),
        );
        let report_path = unique_report_path(&reserved, "iphone-camera-report.json");
        let report_file = extracted_path(extraction_dir, &report_path);
        let report_bytes =
            serde_json::to_vec_pretty(capture).map_err(|_| PackageError::InvalidArchive)?;
        fs::write(&report_file, &report_bytes)?;
        assets.push(PackageAssetImport {
            source_path: report_file,
            package_paths: vec![report_path],
            original_name: "iphone-camera-report.json".to_owned(),
            media_type: "application/json".to_owned(),
            role: AssetRole::Report,
            source: false,
            expected_sha256: Some(hex_digest(&Sha256::digest(&report_bytes))),
            expected_bytes: Some(report_bytes.len() as u64),
            metadata: serde_json::json!({ "derived_report": "iphone_camera_set" }),
        });
    }
    Ok(())
}

fn unique_report_path(used_paths: &HashSet<String>, filename: &str) -> String {
    let base = format!("__3dmk/derived/{filename}");
    if !used_paths.contains(&base.to_ascii_lowercase()) {
        return base;
    }
    for suffix in 1_u32.. {
        let candidate = format!("__3dmk/derived/{suffix}-{filename}");
        if !used_paths.contains(&candidate.to_ascii_lowercase()) {
            return candidate;
        }
    }
    unreachable!("an unbounded suffix always provides a unique report path")
}

fn inspect_reader<R: Read + Seek>(
    reader: R,
    archive_bytes: u64,
    limits: ArchiveLimits,
) -> Result<ArchiveInventory> {
    if archive_bytes > limits.max_archive_bytes {
        return Err(PackageError::ArchiveTooLarge);
    }
    let mut archive = ZipArchive::new(reader).map_err(|_| PackageError::InvalidArchive)?;
    if archive.len() > limits.max_entries {
        return Err(PackageError::TooManyEntries);
    }

    let mut entries = Vec::with_capacity(archive.len());
    let mut normalized_paths = HashSet::with_capacity(archive.len());
    let mut control_files = BTreeMap::new();
    let mut total_expanded_bytes = 0_u64;
    let mut file_count = 0_usize;
    let mut directory_count = 0_usize;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|_| PackageError::InvalidArchive)?;
        let path = normalize_entry_path(entry.name_raw(), limits)?;
        if !normalized_paths.insert(path.to_ascii_lowercase()) {
            return Err(PackageError::DuplicatePath);
        }
        if entry.encrypted() {
            return Err(PackageError::EncryptedEntry);
        }
        if entry.is_symlink() || is_special_unix_entry(entry.unix_mode()) {
            return Err(PackageError::UnsupportedLink);
        }
        if !matches!(
            entry.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(PackageError::UnsupportedCompression);
        }

        let expanded_bytes = entry.size();
        let compressed_bytes = entry.compressed_size();
        if expanded_bytes > limits.max_entry_expanded_bytes {
            return Err(PackageError::EntryTooLarge);
        }
        total_expanded_bytes = total_expanded_bytes
            .checked_add(expanded_bytes)
            .ok_or(PackageError::ExpandedArchiveTooLarge)?;
        if total_expanded_bytes > limits.max_total_expanded_bytes {
            return Err(PackageError::ExpandedArchiveTooLarge);
        }
        if expanded_bytes > 0
            && (compressed_bytes == 0
                || expanded_bytes > compressed_bytes.saturating_mul(limits.max_compression_ratio))
        {
            return Err(PackageError::CompressionRatioTooHigh);
        }

        let directory = entry.is_dir();
        let (media_type, sha256) = if directory {
            directory_count += 1;
            ("inode/directory".to_owned(), None)
        } else {
            file_count += 1;
            let keep_control_file = is_control_file(&path) && expanded_bytes <= 4 * 1024 * 1024;
            let mut control_bytes = keep_control_file.then(Vec::new);
            let mut first_bytes = Vec::with_capacity(4096);
            let mut hash = Sha256::new();
            let mut observed_bytes = 0_u64;
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let read = entry
                    .read(&mut buffer)
                    .map_err(|_| PackageError::InvalidArchive)?;
                if read == 0 {
                    break;
                }
                observed_bytes = observed_bytes
                    .checked_add(read as u64)
                    .ok_or(PackageError::EntryTooLarge)?;
                if observed_bytes > expanded_bytes
                    || observed_bytes > limits.max_entry_expanded_bytes
                {
                    return Err(PackageError::EntrySizeMismatch);
                }
                hash.update(&buffer[..read]);
                if first_bytes.len() < 4096 {
                    let remaining = 4096 - first_bytes.len();
                    first_bytes.extend_from_slice(&buffer[..read.min(remaining)]);
                }
                if let Some(bytes) = &mut control_bytes {
                    bytes.extend_from_slice(&buffer[..read]);
                }
            }
            if observed_bytes != expanded_bytes {
                return Err(PackageError::EntrySizeMismatch);
            }
            if let Some(bytes) = control_bytes {
                control_files.insert(path.to_ascii_lowercase(), bytes);
            }
            let digest = hash.finalize();
            (
                detect_media_type(&path, &first_bytes).to_owned(),
                Some(hex_digest(&digest)),
            )
        };

        entries.push(ArchiveEntry {
            path,
            directory,
            compressed_bytes,
            expanded_bytes,
            compression: compression_name(entry.compression()).to_owned(),
            media_type,
            sha256,
        });
    }

    entries.sort_by(|left, right| left.path.cmp(&right.path));
    let primary_model = resolve_primary_model(&entries, &control_files);
    let adapter_candidates = adapter_candidates(&entries, &control_files, &primary_model);
    let iphone_capture = parse_iphone_capture(&entries, &control_files, limits)?;
    Ok(ArchiveInventory {
        schema_version: ARCHIVE_INVENTORY_SCHEMA_VERSION,
        archive_bytes,
        entry_count: entries.len(),
        file_count,
        directory_count,
        total_expanded_bytes,
        entries,
        adapter_candidates,
        primary_model,
        iphone_capture,
    })
}

fn normalize_entry_path(raw: &[u8], limits: ArchiveLimits) -> Result<String> {
    let raw = std::str::from_utf8(raw).map_err(|_| PackageError::UnsafePath)?;
    if raw.is_empty()
        || raw.starts_with('/')
        || raw.starts_with('\\')
        || raw
            .chars()
            .any(|character| character == '\0' || character.is_control())
    {
        return Err(PackageError::UnsafePath);
    }
    let normalized = raw.replace('\\', "/");
    let path = normalized.strip_suffix('/').unwrap_or(&normalized);
    if path.starts_with('/')
        || path.starts_with("//")
        || path.contains("//")
        || path.as_bytes().get(1) == Some(&b':')
        || path.contains(':')
        || path.len() > limits.max_path_bytes
    {
        return Err(PackageError::UnsafePath);
    }

    let mut components = Vec::new();
    for component in path.split('/') {
        match component {
            "" | ".." => return Err(PackageError::UnsafePath),
            "." => {}
            value => components.push(value),
        }
    }
    if components.is_empty() || components.len() > limits.max_path_depth {
        return Err(PackageError::UnsafePath);
    }
    Ok(components.join("/"))
}

fn is_special_unix_entry(mode: Option<u32>) -> bool {
    let Some(kind) = mode.map(|value| value & 0o170000) else {
        return false;
    };
    !matches!(kind, 0 | 0o040000 | 0o100000)
}

fn compression_name(method: CompressionMethod) -> &'static str {
    match method {
        CompressionMethod::Stored => "stored",
        CompressionMethod::Deflated => "deflated",
        _ => "unsupported",
    }
}

fn is_control_file(path: &str) -> bool {
    matches!(
        path.rsplit('/')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "3dmk-package.json" | "manifest.json" | "session.json" | "rawimages_blocks.json"
    )
}

fn detect_media_type(path: &str, bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"glTF") {
        return "model/gltf-binary";
    }
    if bytes.starts_with(b"ply\n") || bytes.starts_with(b"ply\r\n") {
        return "application/ply";
    }
    if bytes.starts_with(b"LASF") {
        return "application/vnd.las";
    }
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        return "image/png";
    }
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return "image/jpeg";
    }
    let extension = path
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let text = String::from_utf8_lossy(bytes);
    let first_non_whitespace = bytes
        .iter()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace());
    match extension.as_str() {
        "json" if matches!(first_non_whitespace, Some(b'{') | Some(b'[')) => "application/json",
        "gltf" if first_non_whitespace == Some(b'{') => "model/gltf+json",
        "obj"
            if text.lines().any(|line| {
                let line = line.trim_start();
                line.starts_with("v ") || line.starts_with("o ") || line.starts_with("mtllib ")
            }) =>
        {
            "model/obj"
        }
        "3ds" => "application/x-3ds",
        "3mf" => "model/3mf",
        "dae" => "model/vnd.collada+xml",
        "fbx" => "model/fbx",
        "off" => "model/off",
        "u3d" => "model/u3d",
        "x3d" if text.trim_start().starts_with("<") => "model/x3d+xml",
        "mtl"
            if text.lines().any(|line| {
                let line = line.trim_start();
                line.starts_with("newmtl ") || line.starts_with("map_Kd ")
            }) =>
        {
            "model/mtl"
        }
        "stl" if text.trim_start().starts_with("solid ") => "model/stl",
        "log" | "txt" if !bytes.contains(&0) => "text/plain",
        _ => "application/octet-stream",
    }
}

fn resolve_primary_model(
    entries: &[ArchiveEntry],
    control_files: &BTreeMap<String, Vec<u8>>,
) -> PrimaryModelResolution {
    let paths = entries
        .iter()
        .map(|entry| entry.path.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let has_mtl = entries.iter().any(|entry| entry.media_type == "model/mtl");
    let mut candidates = entries
        .iter()
        .filter_map(|entry| {
            let format = match entry.media_type.as_str() {
                "model/gltf-binary" => "glb",
                "model/gltf+json" => "gltf",
                "model/obj" => "obj",
                "model/fbx" => "fbx",
                "model/vnd.collada+xml" => "dae",
                "application/x-3ds" => "3ds",
                "model/3mf" => "3mf",
                "model/off" => "off",
                "model/u3d" => "u3d",
                "model/x3d+xml" => "x3d",
                "application/ply" => "ply",
                "application/vnd.las" => "las",
                "model/stl" => "stl",
                _ => return None,
            };
            let depth = entry.path.matches('/').count() + 1;
            let filename = entry
                .path
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            let mut score = match format {
                "glb" => 40,
                "gltf" => 30,
                "obj" => 25,
                "fbx" | "dae" | "3ds" | "3mf" | "off" | "u3d" | "x3d" => 15,
                "ply" | "las" => 20,
                "stl" => 10,
                _ => 0,
            };
            let mut reasons = vec![format!("detected {format} content")];
            if depth == 1 {
                score += 20;
                reasons.push("archive-root model".to_owned());
            }
            if filename.contains("refined")
                || filename.contains("model")
                || filename.contains("scene")
            {
                score += 15;
                reasons.push("descriptive model filename".to_owned());
            }
            if format == "obj" && has_mtl {
                score += 10;
                reasons.push("material library is present".to_owned());
            }
            Some(ModelCandidate {
                path: entry.path.clone(),
                format: format.to_owned(),
                score,
                reasons,
            })
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.path.cmp(&right.path))
    });
    if candidates.is_empty() {
        return PrimaryModelResolution {
            selected_path: None,
            requires_user_choice: false,
            reason: "No supported model content was detected.".to_owned(),
            candidates,
        };
    }

    for manifest_name in ["3dmk-package.json", "manifest.json"] {
        if let Some(bytes) = control_files.get(manifest_name) {
            if let Ok(manifest) = serde_json::from_slice::<Value>(bytes) {
                for pointer in [
                    "/primary_model_path",
                    "/active_revision/render_path",
                    "/active_revision/scene_path",
                    "/geometry/active_asset",
                    "/model",
                ] {
                    if let Some(declared) = manifest.pointer(pointer).and_then(Value::as_str) {
                        let declared = declared.replace('\\', "/");
                        if paths.contains(&declared.to_ascii_lowercase())
                            && candidates
                                .iter()
                                .any(|candidate| candidate.path.eq_ignore_ascii_case(&declared))
                        {
                            return PrimaryModelResolution {
                                selected_path: Some(declared),
                                requires_user_choice: false,
                                reason: format!("Selected by {manifest_name}."),
                                candidates,
                            };
                        }
                    }
                }
            }
        }
    }

    let iphone_capture = control_files.contains_key("session.json")
        && control_files.contains_key("rawimages_blocks.json");
    if iphone_capture {
        if let Some(candidate) = candidates.iter().find(|candidate| {
            candidate
                .path
                .rsplit('/')
                .next()
                .is_some_and(|name| name.eq_ignore_ascii_case("Refined-Mesh-1.glb"))
        }) {
            return PrimaryModelResolution {
                selected_path: Some(candidate.path.clone()),
                requires_user_choice: false,
                reason:
                    "Selected the refined mesh declared by the supported iPhone capture layout."
                        .to_owned(),
                candidates,
            };
        }
    }

    let top_score = candidates[0].score;
    let top = candidates
        .iter()
        .filter(|candidate| candidate.score == top_score)
        .collect::<Vec<_>>();
    if top.len() == 1 {
        PrimaryModelResolution {
            selected_path: Some(top[0].path.clone()),
            requires_user_choice: false,
            reason: format!(
                "Selected the only highest-ranked model (score {}).",
                top_score
            ),
            candidates,
        }
    } else {
        PrimaryModelResolution {
            selected_path: None,
            requires_user_choice: true,
            reason: format!(
                "{} model candidates share the highest score; choose the intended model.",
                top.len()
            ),
            candidates,
        }
    }
}

fn adapter_candidates(
    entries: &[ArchiveEntry],
    control_files: &BTreeMap<String, Vec<u8>>,
    primary: &PrimaryModelResolution,
) -> Vec<AdapterCandidate> {
    let mut candidates = vec![];
    if control_files.contains_key("3dmk-package.json") {
        candidates.push(AdapterCandidate {
            id: "3dmk_package".to_owned(),
            confidence: 100,
            reason: "Root 3dmk-package.json is present.".to_owned(),
        });
    }
    if control_files.contains_key("session.json")
        && control_files.contains_key("rawimages_blocks.json")
        && entries.iter().any(|entry| {
            entry
                .path
                .rsplit('/')
                .next()
                .is_some_and(|name| name.eq_ignore_ascii_case("Refined-Mesh-1.glb"))
        })
    {
        candidates.push(AdapterCandidate {
            id: "iphone_capture".to_owned(),
            confidence: 100,
            reason: "Session, raw-image descriptors, and refined mesh are present.".to_owned(),
        });
    }
    if control_files.contains_key("manifest.json") {
        candidates.push(AdapterCandidate {
            id: "legacy_3dmk_package".to_owned(),
            confidence: 90,
            reason: "Legacy manifest.json is present.".to_owned(),
        });
    }
    if !primary.candidates.is_empty() {
        candidates.push(AdapterCandidate {
            id: "model_package".to_owned(),
            confidence: 70,
            reason: "One or more supported model files are present.".to_owned(),
        });
    }
    candidates.sort_by(|left, right| right.confidence.cmp(&left.confidence));
    candidates
}

fn parse_iphone_capture(
    entries: &[ArchiveEntry],
    control_files: &BTreeMap<String, Vec<u8>>,
    limits: ArchiveLimits,
) -> Result<Option<IphoneCaptureReport>> {
    let (Some(session_bytes), Some(image_descriptor_bytes)) = (
        control_files.get("session.json"),
        control_files.get("rawimages_blocks.json"),
    ) else {
        return Ok(None);
    };
    let session: SessionDescriptor =
        serde_json::from_slice(session_bytes).map_err(|_| PackageError::InvalidCaptureMetadata)?;
    let image_descriptors: Vec<RawImageDescriptor> = serde_json::from_slice(image_descriptor_bytes)
        .map_err(|_| PackageError::InvalidCaptureMetadata)?;
    let image_directory =
        normalize_entry_path(session.texture_images.directory_name.as_bytes(), limits)?;
    let image_prefix = format!("{}/", image_directory.to_ascii_lowercase());
    let paths = entries
        .iter()
        .map(|entry| entry.path.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let observed_images = entries
        .iter()
        .filter(|entry| {
            entry.path.to_ascii_lowercase().starts_with(&image_prefix)
                && entry.media_type == "image/jpeg"
        })
        .count();
    let observed_camera_json = entries
        .iter()
        .filter(|entry| {
            entry.path.to_ascii_lowercase().starts_with(&image_prefix)
                && entry.media_type == "application/json"
        })
        .count();

    let mut cameras = Vec::with_capacity(image_descriptors.len());
    let mut missing_records = vec![];
    for descriptor in image_descriptors {
        let image_path = normalize_entry_path(descriptor.file_path.as_bytes(), limits)?;
        let camera_json_path = match image_path.rsplit_once('.') {
            Some((stem, _)) => format!("{stem}.json"),
            None => format!("{image_path}.json"),
        };
        let image_present = paths.contains(&image_path.to_ascii_lowercase());
        let camera_json_present = paths.contains(&camera_json_path.to_ascii_lowercase());
        if !image_present || !camera_json_present {
            missing_records.push(MissingImageRecord {
                index: descriptor.index,
                image_path: image_path.clone(),
                missing_image: !image_present,
                missing_camera_json: !camera_json_present,
            });
        }
        let world_from_camera = world_from_camera(&descriptor.pose);
        let calibration_valid = valid_camera(
            descriptor.dimensions,
            descriptor.pose.focal_length,
            descriptor.pose.principal_point,
            &world_from_camera,
        );
        cameras.push(CanonicalCamera {
            id: descriptor.id,
            index: descriptor.index,
            image_path,
            camera_json_path,
            dimensions: descriptor.dimensions,
            focal_pixels: descriptor.pose.focal_length,
            principal_point_pixels: descriptor.pose.principal_point,
            world_from_camera,
            image_present,
            camera_json_present,
            calibration_valid,
        });
    }
    cameras.sort_by_key(|camera| camera.index);
    missing_records.sort_by_key(|record| record.index);
    let valid_image_pairs = cameras
        .iter()
        .filter(|camera| camera.image_present && camera.camera_json_present)
        .count();
    let calibration_valid = cameras.iter().all(|camera| camera.calibration_valid);
    let primary_model_path = entries
        .iter()
        .find(|entry| {
            entry
                .path
                .rsplit('/')
                .next()
                .is_some_and(|name| name.eq_ignore_ascii_case("Refined-Mesh-1.glb"))
        })
        .map(|entry| entry.path.clone())
        .ok_or(PackageError::InvalidCaptureMetadata)?;

    let mut warnings = vec![];
    if session.texture_images.count != cameras.len() {
        warnings.push(CaptureWarning {
            code: "capture_descriptor_count_mismatch".to_owned(),
            message: format!(
                "Session.json expects {} image records, but RawImages_blocks.json declares {}.",
                session.texture_images.count,
                cameras.len()
            ),
        });
    }
    if valid_image_pairs != session.texture_images.count {
        warnings.push(CaptureWarning {
            code: "incomplete_capture_images".to_owned(),
            message: format!(
                "Session.json expects {} image records; {} complete image/JSON pairs are present and {} are missing.",
                session.texture_images.count,
                valid_image_pairs,
                session.texture_images.count.saturating_sub(valid_image_pairs)
            ),
        });
    }
    let invalid_cameras = cameras
        .iter()
        .filter(|camera| !camera.calibration_valid)
        .count();
    if invalid_cameras > 0 {
        warnings.push(CaptureWarning {
            code: "invalid_camera_calibration".to_owned(),
            message: format!(
                "{invalid_cameras} camera records have invalid intrinsics or non-rigid transforms."
            ),
        });
    }

    Ok(Some(IphoneCaptureReport {
        schema_version: 1,
        capture_id: session.uuid,
        name: session.name,
        primary_model_path,
        expected_image_records: session.texture_images.count,
        descriptor_records: cameras.len(),
        observed_images,
        observed_camera_json,
        valid_image_pairs,
        missing_records,
        raw_points: session.raw_points.map(point_summary),
        cloud_points: session
            .cloud
            .map(|cloud| point_summary(cloud.point_descriptor)),
        camera_convention: CameraConvention {
            transform:
                "4x4 row-major world_from_camera; translation is camera centre in scan metres"
                    .to_owned(),
            local_axes: "+X right, +Y up, +Z back; optical forward is -Z".to_owned(),
            image_origin: "top-left; +u right, +v down".to_owned(),
            intrinsic_units: "pixels at the declared image dimensions".to_owned(),
            projection: "p=R_wc^T(P-C); depth=-p.z; u=fx*p.x/depth+cx; v=fy*(-p.y)/depth+cy"
                .to_owned(),
        },
        calibration_valid,
        cameras,
        warnings,
    }))
}

fn point_summary(descriptor: RawPointDescriptor) -> PointDescriptorSummary {
    PointDescriptorSummary {
        point_count: descriptor.point_count,
        block_count: descriptor.block_count,
        has_colors: descriptor.has_colors,
        has_normals: descriptor.has_normals,
        format: descriptor.format,
    }
}

fn world_from_camera(pose: &RawCameraPose) -> [[f64; 4]; 4] {
    let rotation = pose.rotation_matrix;
    [
        [rotation[0], rotation[3], rotation[6], pose.translation[0]],
        [rotation[1], rotation[4], rotation[7], pose.translation[1]],
        [rotation[2], rotation[5], rotation[8], pose.translation[2]],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn valid_camera(
    dimensions: [u32; 2],
    focal: [f64; 2],
    principal: [f64; 2],
    transform: &[[f64; 4]; 4],
) -> bool {
    if dimensions.contains(&0)
        || focal
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
        || principal.iter().any(|value| !value.is_finite())
        || principal[0] < 0.0
        || principal[0] > dimensions[0] as f64
        || principal[1] < 0.0
        || principal[1] > dimensions[1] as f64
        || transform.iter().flatten().any(|value| !value.is_finite())
    {
        return false;
    }
    for column in 0..3 {
        let length = (0..3)
            .map(|row| transform[row][column].powi(2))
            .sum::<f64>()
            .sqrt();
        if (length - 1.0).abs() > 0.02 {
            return false;
        }
        for other in (column + 1)..3 {
            let dot = (0..3)
                .map(|row| transform[row][column] * transform[row][other])
                .sum::<f64>();
            if dot.abs() > 0.02 {
                return false;
            }
        }
    }
    let determinant = transform[0][0]
        * (transform[1][1] * transform[2][2] - transform[1][2] * transform[2][1])
        - transform[0][1] * (transform[1][0] * transform[2][2] - transform[1][2] * transform[2][0])
        + transform[0][2] * (transform[1][0] * transform[2][1] - transform[1][1] * transform[2][0]);
    (determinant - 1.0).abs() <= 0.02
}

fn dot_column(transform: &[[f64; 4]; 4], column: usize, vector: [f64; 3]) -> f64 {
    (0..3).map(|row| transform[row][column] * vector[row]).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::{write::SimpleFileOptions, ZipWriter};

    fn archive_with(entries: &[(&str, &[u8])], deflated: bool) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        let method = if deflated {
            CompressionMethod::Deflated
        } else {
            CompressionMethod::Stored
        };
        for (name, bytes) in entries {
            writer
                .start_file(
                    *name,
                    SimpleFileOptions::default().compression_method(method),
                )
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn inspect_bytes(bytes: Vec<u8>, limits: ArchiveLimits) -> Result<ArchiveInventory> {
        let length = bytes.len() as u64;
        inspect_reader(Cursor::new(bytes), length, limits)
    }

    #[test]
    fn inventory_sniffs_content_and_obeys_manifest_primary_model() {
        let manifest = br#"{"geometry":{"active_asset":"geometry/model.obj"}}"#;
        let bytes = archive_with(
            &[
                ("manifest.json", manifest),
                ("geometry/model.obj", b"mtllib model.mtl\nv 0 0 0\n"),
                (
                    "geometry/model.mtl",
                    b"newmtl room\nmap_Kd ../texture.jpg\n",
                ),
                ("texture.jpg", &[0xff, 0xd8, 0xff, 0xd9]),
            ],
            true,
        );
        let inventory = inspect_bytes(bytes, ArchiveLimits::default()).unwrap();
        assert_eq!(inventory.entry_count, 4);
        assert_eq!(
            inventory.primary_model.selected_path.as_deref(),
            Some("geometry/model.obj")
        );
        assert!(inventory.primary_model.reason.contains("manifest.json"));
        assert_eq!(
            inventory
                .entries
                .iter()
                .find(|entry| entry.path == "texture.jpg")
                .unwrap()
                .media_type,
            "image/jpeg"
        );
    }

    #[test]
    fn unsafe_duplicate_and_link_entries_are_rejected() {
        let traversal = archive_with(&[("../outside.obj", b"v 0 0 0\n")], false);
        assert!(matches!(
            inspect_bytes(traversal, ArchiveLimits::default()),
            Err(PackageError::UnsafePath)
        ));

        let duplicate = archive_with(
            &[("Model.obj", b"v 0 0 0\n"), ("model.obj", b"v 0 0 0\n")],
            false,
        );
        assert!(matches!(
            inspect_bytes(duplicate, ArchiveLimits::default()),
            Err(PackageError::DuplicatePath)
        ));

        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        writer
            .add_symlink("model.obj", "outside.obj", SimpleFileOptions::default())
            .unwrap();
        let symlink = writer.finish().unwrap().into_inner();
        assert!(matches!(
            inspect_bytes(symlink, ArchiveLimits::default()),
            Err(PackageError::UnsupportedLink)
        ));
    }

    #[test]
    fn archive_budgets_are_enforced_before_publish() {
        let bytes = archive_with(&[("large.txt", &vec![b'x'; 128 * 1024])], true);
        let ratio_limits = ArchiveLimits {
            max_compression_ratio: 2,
            ..ArchiveLimits::default()
        };
        assert!(matches!(
            inspect_bytes(bytes, ratio_limits),
            Err(PackageError::CompressionRatioTooHigh)
        ));

        let bytes = archive_with(&[("one.txt", b"1"), ("two.txt", b"2")], false);
        let count_limits = ArchiveLimits {
            max_entries: 1,
            ..ArchiveLimits::default()
        };
        assert!(matches!(
            inspect_bytes(bytes, count_limits),
            Err(PackageError::TooManyEntries)
        ));
    }

    #[test]
    fn iphone_capture_reports_missing_pairs_and_uses_documented_projection() {
        let session = br#"{
            "name":"Room", "uuid":"capture-1",
            "textureImageDescriptor":{"directoryName":"RawImages","count":2},
            "rawPointsDescriptor":{"pointCount":10,"blockCount":1,"hasColors":true,"hasNormals":false,"format":"floatposition"},
            "cloudDescriptor":{"pointDescriptor":{"pointCount":5,"blockCount":1,"hasColors":true,"hasNormals":false,"format":"floatposition"}}
        }"#;
        let blocks = br#"[
            {"dimensions":[100,80],"file_path":"RawImages/1.jpg","id":"camera-1","index":1,"pose":{"focal_length":[50,50],"principal_point":[50,40],"rotation_matrix":[1,0,0,0,1,0,0,0,1],"translation":[10,2,-3]}},
            {"dimensions":[100,80],"file_path":"RawImages/2.jpg","id":"camera-2","index":2,"pose":{"focal_length":[50,50],"principal_point":[50,40],"rotation_matrix":[1,0,0,0,1,0,0,0,1],"translation":[10,2,-3]}}
        ]"#;
        let bytes = archive_with(
            &[
                ("Session.json", session),
                ("RawImages_blocks.json", blocks),
                ("Refined-Mesh-1.glb", b"glTF"),
                ("RawImages/1.jpg", &[0xff, 0xd8, 0xff, 0xd9]),
                ("RawImages/1.json", b"{}"),
            ],
            false,
        );
        let inventory = inspect_bytes(bytes, ArchiveLimits::default()).unwrap();
        let capture = inventory.iphone_capture.unwrap();
        assert_eq!(capture.expected_image_records, 2);
        assert_eq!(capture.descriptor_records, 2);
        assert_eq!(capture.valid_image_pairs, 1);
        assert_eq!(capture.missing_records.len(), 1);
        assert_eq!(capture.raw_points.unwrap().point_count, 10);
        assert!(capture.calibration_valid);
        let camera = &capture.cameras[0];
        assert_eq!(camera.project_world([10.0, 2.0, -5.0]), Some([50.0, 40.0]));
        assert_eq!(camera.project_world([11.0, 3.0, -5.0]), Some([75.0, 15.0]));
        assert_eq!(camera.project_world([10.0, 2.0, -1.0]), None);
    }

    #[test]
    fn malformed_zip_is_rejected() {
        assert!(matches!(
            inspect_bytes(b"not a zip".to_vec(), ArchiveLimits::default()),
            Err(PackageError::InvalidArchive)
        ));
    }

    #[test]
    fn preparation_extracts_and_groups_assets_with_a_verified_digest() {
        let root = std::env::temp_dir().join(format!("3dmk-package-test-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let archive_path = root.join("room.zip");
        let model = b"glTF";
        let texture = &[0xff, 0xd8, 0xff, 0xd9];
        fs::write(
            &archive_path,
            archive_with(
                &[
                    ("room.glb", model),
                    ("textures/a.jpg", texture),
                    ("textures/b.jpg", texture),
                ],
                false,
            ),
        )
        .unwrap();
        let prepared = prepare_archive(
            &archive_path,
            &root.join("staging"),
            None,
            ArchiveLimits::default(),
        )
        .unwrap();

        assert_eq!(prepared.selected_model_path, "room.glb");
        let texture_asset = prepared
            .assets
            .iter()
            .find(|asset| asset.role == AssetRole::Texture)
            .unwrap();
        assert_eq!(texture_asset.package_paths.len(), 2);
        assert!(texture_asset.source_path.is_file());
        let staging = prepared.extraction_dir.clone();
        drop(prepared);
        assert!(!staging.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn preparation_requires_a_choice_for_tied_primary_models() {
        let root = std::env::temp_dir().join(format!("3dmk-package-choice-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let archive_path = root.join("room.zip");
        fs::write(
            &archive_path,
            archive_with(&[("a.glb", b"glTF"), ("b.glb", b"glTF")], false),
        )
        .unwrap();
        assert!(matches!(
            prepare_archive(
                &archive_path,
                &root.join("staging"),
                None,
                ArchiveLimits::default(),
            ),
            Err(PackageError::PrimaryModelChoiceRequired)
        ));
        let prepared = prepare_archive(
            &archive_path,
            &root.join("staging"),
            Some("b.glb"),
            ArchiveLimits::default(),
        )
        .unwrap();
        assert_eq!(prepared.selected_model_path, "b.glb");
        drop(prepared);
        let _ = fs::remove_dir_all(root);
    }
}
