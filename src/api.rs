use axum::{
    body::Body,
    extract::{multipart::Field, DefaultBodyLimit, Multipart, Path as AxumPath, State},
    http::{
        header::{HeaderValue, CONTENT_LENGTH, CONTENT_TYPE},
        Request, StatusCode,
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use tower_http::services::ServeDir;
use uuid::Uuid;

use crate::{
    ai_vision, cad_engine, image_refinement,
    jobs::{JobError, JobRegistry},
    packages::{self, ArchiveInventory, ArchiveLimits, PackageError},
    perception, point_cloud,
    projects::{
        Asset, AssetRole, AttributeContract, JobRecord, PackageAssetImport, PackageSceneImport,
        Project, ProjectError, ProjectStore, ProjectWarning, Revision, RootSceneImport, SceneKind,
        DEFAULT_MAX_ASSET_BYTES,
    },
};

static LEGACY_API_REQUESTS: AtomicU64 = AtomicU64::new(0);
const MAX_CALIBRATED_ROUTE_BODY_BYTES: usize = 384 * 1024 * 1024;
const MAX_CALIBRATED_MULTIPART_PAYLOAD_BYTES: usize = 376 * 1024 * 1024;
const MAX_CALIBRATED_CLOUD_BYTES: usize = 128 * 1024 * 1024;
const MAX_CALIBRATED_CAMERA_REPORT_BYTES: usize = 4 * 1024 * 1024;
const MAX_CALIBRATED_MANIFEST_BYTES: usize = 512 * 1024;
const MAX_CALIBRATED_PROVENANCE_BYTES: usize = 16 * 1024;
const MAX_CALIBRATED_OPTIONS_BYTES: usize = 64 * 1024;
const MAX_CALIBRATED_COMPRESSED_IMAGE_BYTES: usize = 192 * 1024 * 1024;
const MAX_VWM_PERCEPTION_ROUTE_BODY_BYTES: usize =
    perception::MAX_VWM_PERCEPTION_IMAGE_BYTES + 512 * 1024;

pub fn create_router(
    output_dir: PathBuf,
    public_dir: PathBuf,
    project_data_dir: PathBuf,
    _port: u16,
) -> crate::projects::Result<Router> {
    let projects = ProjectStore::open(project_data_dir)?;
    let state = AppState {
        output_dir,
        public_dir,
        jobs: JobRegistry::open(projects.clone(), 2)?,
        projects,
    };

    Ok(Router::new()
        .route("/api/v1/capabilities", get(handle_capabilities))
        .route(
            "/api/v1/projects",
            get(handle_list_projects).post(handle_create_project),
        )
        .route("/api/v1/projects/import", post(handle_import_project))
        .route(
            "/api/v1/projects/import-package",
            post(handle_import_package),
        )
        .route("/api/v1/packages/inspect", post(handle_inspect_package))
        .route("/api/v1/projects/:project_id", get(handle_get_project))
        .route(
            "/api/v1/projects/:project_id/assets/:asset_id",
            get(handle_get_asset),
        )
        .route(
            "/api/v1/projects/:project_id/assets",
            get(handle_list_assets),
        )
        .route(
            "/api/v1/projects/:project_id/export",
            get(handle_export_project),
        )
        .route(
            "/api/v1/projects/:project_id/revisions",
            get(handle_list_revisions),
        )
        .route(
            "/api/v1/projects/:project_id/active-revision",
            post(handle_set_active_revision),
        )
        .route(
            "/api/v1/jobs/:job_id",
            get(handle_get_job).delete(handle_cancel_job),
        )
        .route("/api/health", get(handle_health))
        .route("/api/pdf-to-3d", post(handle_pdf_to_3d))
        .route("/api/poisson-reconstruct", post(handle_poisson))
        .route("/api/point-cloud-to-mesh", post(handle_poisson))
        .route(
            "/api/point-cloud-analyze",
            post(handle_point_cloud_analysis),
        )
        .route(
            "/api/vwm-geometry-analyze",
            post(handle_vwm_geometry_analysis).layer(DefaultBodyLimit::max(256 * 1024 * 1024)),
        )
        .route(
            "/api/vwm-perception",
            post(handle_vwm_perception)
                .layer(DefaultBodyLimit::max(MAX_VWM_PERCEPTION_ROUTE_BODY_BYTES)),
        )
        .route(
            "/api/flat-surface-correct",
            post(handle_flat_surface_correction),
        )
        .route("/api/mesh-smooth", post(handle_mesh_smooth))
        .route(
            "/api/image-assisted-refinement",
            post(handle_image_assisted_refinement),
        )
        .route(
            "/api/calibrated-photo-project",
            post(handle_calibrated_photo_project)
                .layer(DefaultBodyLimit::max(MAX_CALIBRATED_ROUTE_BODY_BYTES)),
        )
        .route(
            "/api/model-floorplan-recognize",
            post(handle_model_floorplan_recognition),
        )
        .nest_service(
            "/output",
            ServeDir::new(state.output_dir.clone()).append_index_html_on_directories(false),
        )
        .nest_service(
            "/",
            ServeDir::new(state.public_dir.clone()).append_index_html_on_directories(true),
        )
        .layer(DefaultBodyLimit::max(1024 * 1024 * 1024))
        .layer(middleware::from_fn(track_legacy_api))
        .with_state(state))
}

#[derive(Clone)]
struct AppState {
    output_dir: PathBuf,
    public_dir: PathBuf,
    projects: ProjectStore,
    jobs: JobRegistry,
}

#[derive(Debug, Deserialize)]
struct CreateProjectRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct SetActiveRevisionRequest {
    revision_id: String,
}

#[derive(Debug, Serialize)]
struct ImportSummary {
    vertices: usize,
    faces: usize,
    scene_kind: SceneKind,
    attributes: AttributeContract,
    original_units: &'static str,
}

#[derive(Debug, Serialize)]
struct ImportProjectResponse {
    project: Project,
    source_asset: Asset,
    root_revision: Revision,
    summary: ImportSummary,
}

#[derive(Debug, Serialize)]
struct InspectPackageResponse {
    file_name: String,
    inventory: ArchiveInventory,
}

#[derive(Debug, Serialize)]
struct ImportPackageResponse {
    project: Project,
    assets: Vec<Asset>,
    root_revision: Revision,
    summary: ImportSummary,
    inventory: ArchiveInventory,
}

struct ImportTempFile {
    path: PathBuf,
}

impl Drop for ImportTempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

struct PendingOutputFile {
    path: PathBuf,
    preserve: bool,
}

impl PendingOutputFile {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            preserve: false,
        }
    }

    fn preserve(mut self) {
        self.preserve = true;
    }
}

impl Drop for PendingOutputFile {
    fn drop(&mut self) {
        if !self.preserve {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

async fn stream_multipart_file(
    field: &mut Field<'_>,
    path: &Path,
    max_bytes: u64,
    too_large: fn() -> ApiErrorResponse,
) -> std::result::Result<u64, ApiErrorResponse> {
    let mut output = tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .await
        .map_err(|_| project_store_io_error())?;
    let mut byte_length = 0_u64;
    while let Some(chunk) = field.chunk().await.map_err(|_| {
        ApiErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "invalid_upload",
            "The upload ended unexpectedly.",
            false,
        )
    })? {
        byte_length = byte_length
            .checked_add(chunk.len() as u64)
            .ok_or_else(too_large)?;
        if byte_length > max_bytes {
            return Err(too_large());
        }
        output
            .write_all(&chunk)
            .await
            .map_err(|_| project_store_io_error())?;
    }
    output
        .sync_all()
        .await
        .map_err(|_| project_store_io_error())?;
    Ok(byte_length)
}

#[derive(Debug, Serialize)]
struct ApiErrorBody {
    code: &'static str,
    message: &'static str,
    retryable: bool,
}

#[derive(Debug)]
struct ApiErrorResponse {
    status: StatusCode,
    body: ApiErrorBody,
}

impl ApiErrorResponse {
    fn new(status: StatusCode, code: &'static str, message: &'static str, retryable: bool) -> Self {
        Self {
            status,
            body: ApiErrorBody {
                code,
                message,
                retryable,
            },
        }
    }
}

impl IntoResponse for ApiErrorResponse {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.body }))).into_response()
    }
}

async fn handle_create_project(
    State(state): State<AppState>,
    Json(request): Json<CreateProjectRequest>,
) -> std::result::Result<(StatusCode, Json<Project>), ApiErrorResponse> {
    let project = state
        .projects
        .create_project(&request.name)
        .map_err(project_api_error)?;
    Ok((StatusCode::CREATED, Json(project)))
}

async fn handle_import_project(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> std::result::Result<(StatusCode, Json<ImportProjectResponse>), ApiErrorResponse> {
    let import_dir = state.projects.root().join("import-temp");
    tokio::fs::create_dir_all(&import_dir)
        .await
        .map_err(|_| project_store_io_error())?;

    let mut upload: Option<(ImportTempFile, String, String, String)> = None;
    let mut project_name = None;
    let mut requested_units = None;

    while let Some(mut field) = multipart.next_field().await.map_err(|_| {
        ApiErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "invalid_multipart",
            "The import form could not be read.",
            false,
        )
    })? {
        match field.name().unwrap_or_default() {
            "file" => {
                if upload.is_some() {
                    return Err(ApiErrorResponse::new(
                        StatusCode::BAD_REQUEST,
                        "multiple_models",
                        "Import one model file at a time or use a supported package.",
                        false,
                    ));
                }
                let original_name = safe_filename(field.file_name().unwrap_or("model"));
                let extension = Path::new(&original_name)
                    .extension()
                    .and_then(|value| value.to_str())
                    .map(str::to_ascii_lowercase)
                    .unwrap_or_default();
                if !direct_model_extension_supported(&extension) {
                    return Err(ApiErrorResponse::new(
                        StatusCode::UNSUPPORTED_MEDIA_TYPE,
                        "unsupported_model_format",
                        "Phase 1 import accepts GLB, GLTF, PLY, OBJ, STL, FBX, DAE, 3DS, 3MF, OFF, U3D, and X3D. Use a package for dependent files.",
                        false,
                    ));
                }
                let media_type = field
                    .content_type()
                    .map(str::to_owned)
                    .unwrap_or_else(|| model_media_type(&extension).to_owned());
                let temp = ImportTempFile {
                    path: import_dir.join(format!("{}.{}", Uuid::new_v4(), extension)),
                };
                stream_multipart_file(
                    &mut field,
                    &temp.path,
                    DEFAULT_MAX_ASSET_BYTES,
                    asset_too_large_api_error,
                )
                .await?;
                upload = Some((temp, original_name, extension, media_type));
            }
            "name" => {
                project_name = Some(field.text().await.map_err(|_| {
                    ApiErrorResponse::new(
                        StatusCode::BAD_REQUEST,
                        "invalid_project_name",
                        "The project name could not be read.",
                        false,
                    )
                })?);
            }
            "units" => {
                requested_units = Some(field.text().await.map_err(|_| {
                    ApiErrorResponse::new(
                        StatusCode::BAD_REQUEST,
                        "invalid_units",
                        "The import units could not be read.",
                        false,
                    )
                })?);
            }
            _ => {}
        }
    }

    let (upload, original_name, extension, media_type) = upload.ok_or_else(|| {
        ApiErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "missing_model",
            "Choose a GLB, GLTF, PLY, OBJ, STL, FBX, DAE, 3DS, 3MF, OFF, U3D, or X3D model to import.",
            false,
        )
    })?;
    let units = validated_import_units(&extension, requested_units.as_deref())?;
    let validation_path = upload.path.clone();
    let scene = tokio::task::spawn_blocking(move || vwm_io::load_scene(&validation_path))
        .await
        .map_err(|_| {
            ApiErrorResponse::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "model_validation_failed",
                "The model validator stopped unexpectedly.",
                true,
            )
        })?
        .map_err(|_| {
            ApiErrorResponse::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_model",
                "The model could not be decoded as valid geometry.",
                false,
            )
        })?;
    let scene_kind = canonical_scene_kind(&scene)?;
    let attributes = canonical_scene_attributes(&scene);
    let faces = scene.indices.as_ref().map_or(0, Vec::len);
    let project_name = project_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            Path::new(&original_name)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("Imported model")
                .to_owned()
        });
    let mut warnings = vec![];
    if extension == "obj" {
        warnings.push(ProjectWarning {
            code: "external_dependencies_not_imported".to_owned(),
            message: "This single-file import preserves the OBJ bytes but does not include external MTL or texture files; use a ZIP package for dependent assets.".to_owned(),
        });
    }
    let file = std::fs::File::open(&upload.path).map_err(|_| project_store_io_error())?;
    let (project, source_asset, root_revision) = state
        .projects
        .import_root_scene(
            RootSceneImport {
                project_name,
                original_name,
                media_type,
                scene_kind,
                attributes: attributes.clone(),
                original_units: Some(units.to_owned()),
                warnings,
            },
            file,
        )
        .map_err(project_api_error)?;

    Ok((
        StatusCode::CREATED,
        Json(ImportProjectResponse {
            project,
            source_asset,
            root_revision,
            summary: ImportSummary {
                vertices: scene.vertices.len(),
                faces,
                scene_kind,
                attributes,
                original_units: units,
            },
        }),
    ))
}

async fn handle_inspect_package(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> std::result::Result<Json<InspectPackageResponse>, ApiErrorResponse> {
    let import_dir = state.projects.root().join("import-temp");
    tokio::fs::create_dir_all(&import_dir)
        .await
        .map_err(|_| project_store_io_error())?;
    let mut upload = None;
    while let Some(mut field) = multipart.next_field().await.map_err(|_| {
        ApiErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "invalid_multipart",
            "The package form could not be read.",
            false,
        )
    })? {
        if field.name().unwrap_or_default() != "file" {
            continue;
        }
        if upload.is_some() {
            return Err(ApiErrorResponse::new(
                StatusCode::BAD_REQUEST,
                "multiple_packages",
                "Inspect one ZIP package at a time.",
                false,
            ));
        }
        let file_name = safe_filename(field.file_name().unwrap_or("package.zip"));
        if !file_name.to_ascii_lowercase().ends_with(".zip") {
            return Err(ApiErrorResponse::new(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported_package_format",
                "Choose a ZIP package.",
                false,
            ));
        }
        let temp = ImportTempFile {
            path: import_dir.join(format!("{}.zip", Uuid::new_v4())),
        };
        let limits = ArchiveLimits::default();
        stream_multipart_file(
            &mut field,
            &temp.path,
            limits.max_archive_bytes,
            archive_too_large_api_error,
        )
        .await?;
        upload = Some((temp, file_name, limits));
    }
    let (upload, file_name, limits) = upload.ok_or_else(|| {
        ApiErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "missing_package",
            "Choose a ZIP package to inspect.",
            false,
        )
    })?;
    let path = upload.path.clone();
    let inventory = tokio::task::spawn_blocking(move || packages::inspect_archive(&path, limits))
        .await
        .map_err(|_| {
            ApiErrorResponse::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "package_inspector_failed",
                "The package inspector stopped unexpectedly.",
                true,
            )
        })?
        .map_err(package_api_error)?;
    Ok(Json(InspectPackageResponse {
        file_name,
        inventory,
    }))
}

async fn handle_import_package(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> std::result::Result<(StatusCode, Json<ImportPackageResponse>), ApiErrorResponse> {
    let import_dir = state.projects.root().join("import-temp");
    tokio::fs::create_dir_all(&import_dir)
        .await
        .map_err(|_| project_store_io_error())?;
    let mut upload = None;
    let mut project_name = None;
    let mut requested_units = None;
    let mut requested_model = None;

    while let Some(mut field) = multipart.next_field().await.map_err(|_| {
        ApiErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "invalid_multipart",
            "The package form could not be read.",
            false,
        )
    })? {
        match field.name().unwrap_or_default() {
            "file" => {
                if upload.is_some() {
                    return Err(ApiErrorResponse::new(
                        StatusCode::BAD_REQUEST,
                        "multiple_packages",
                        "Import one ZIP package at a time.",
                        false,
                    ));
                }
                let file_name = safe_filename(field.file_name().unwrap_or("package.zip"));
                if !file_name.to_ascii_lowercase().ends_with(".zip") {
                    return Err(ApiErrorResponse::new(
                        StatusCode::UNSUPPORTED_MEDIA_TYPE,
                        "unsupported_package_format",
                        "Choose a ZIP package.",
                        false,
                    ));
                }
                let temp = ImportTempFile {
                    path: import_dir.join(format!("{}.zip", Uuid::new_v4())),
                };
                let limits = ArchiveLimits::default();
                stream_multipart_file(
                    &mut field,
                    &temp.path,
                    limits.max_archive_bytes,
                    archive_too_large_api_error,
                )
                .await?;
                upload = Some((temp, file_name, limits));
            }
            "name" => {
                project_name = Some(field.text().await.map_err(|_| {
                    ApiErrorResponse::new(
                        StatusCode::BAD_REQUEST,
                        "invalid_project_name",
                        "The project name could not be read.",
                        false,
                    )
                })?);
            }
            "units" => {
                requested_units = Some(field.text().await.map_err(|_| {
                    ApiErrorResponse::new(
                        StatusCode::BAD_REQUEST,
                        "invalid_units",
                        "The import units could not be read.",
                        false,
                    )
                })?);
            }
            "primary_model" => {
                requested_model = Some(field.text().await.map_err(|_| {
                    ApiErrorResponse::new(
                        StatusCode::BAD_REQUEST,
                        "invalid_primary_model",
                        "The selected model path could not be read.",
                        false,
                    )
                })?);
            }
            _ => {}
        }
    }

    let (upload, file_name, limits) = upload.ok_or_else(|| {
        ApiErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "missing_package",
            "Choose a ZIP package to import.",
            false,
        )
    })?;
    let archive_path = upload.path.clone();
    let extraction_root = state.projects.root().join("package-staging");
    let selected_model = requested_model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let prepared = tokio::task::spawn_blocking(move || {
        packages::prepare_archive(
            &archive_path,
            &extraction_root,
            selected_model.as_deref(),
            limits,
        )
    })
    .await
    .map_err(|_| {
        ApiErrorResponse::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "package_preparer_failed",
            "The package preparation worker stopped unexpectedly.",
            true,
        )
    })?
    .map_err(package_api_error)?;

    let model_asset = prepared
        .assets
        .iter()
        .find(|asset| asset.role == AssetRole::SourceModel)
        .ok_or_else(|| {
            ApiErrorResponse::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "missing_primary_model",
                "The selected primary model was not extracted.",
                false,
            )
        })?;
    let model_path = model_asset.source_path.clone();
    let model_extension = Path::new(&prepared.selected_model_path)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let units = validated_import_units(&model_extension, requested_units.as_deref())?;
    let scene = tokio::task::spawn_blocking(move || vwm_io::load_scene(&model_path))
        .await
        .map_err(|_| {
            ApiErrorResponse::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "model_validation_failed",
                "The package model validator stopped unexpectedly.",
                true,
            )
        })?
        .map_err(|_| {
            ApiErrorResponse::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_model",
                "The selected package model could not be decoded as valid geometry.",
                false,
            )
        })?;
    let scene_kind = canonical_scene_kind(&scene)?;
    let attributes = canonical_scene_attributes(&scene);
    let faces = scene.indices.as_ref().map_or(0, Vec::len);
    let project_name = project_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            Path::new(&file_name)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("Imported package")
                .to_owned()
        });
    let warnings = prepared
        .inventory
        .iphone_capture
        .as_ref()
        .map(|capture| {
            capture
                .warnings
                .iter()
                .map(|warning| ProjectWarning {
                    code: warning.code.clone(),
                    message: warning.message.clone(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut assets = prepared.assets.clone();
    assets.push(PackageAssetImport {
        source_path: upload.path.clone(),
        package_paths: vec![format!("__3dmk/source/{file_name}")],
        original_name: file_name.clone(),
        media_type: "application/zip".to_owned(),
        role: AssetRole::SourcePackage,
        source: true,
        expected_sha256: None,
        expected_bytes: Some(prepared.inventory.archive_bytes),
        metadata: json!({
            "archive_inventory_schema": prepared.inventory.schema_version,
            "entry_count": prepared.inventory.entry_count,
        }),
    });
    let imported = state
        .projects
        .import_package_scene(PackageSceneImport {
            project_name,
            original_units: Some(units.to_owned()),
            warnings,
            primary_package_path: prepared.selected_model_path.clone(),
            scene_kind,
            attributes: attributes.clone(),
            assets,
        })
        .map_err(project_api_error)?;

    Ok((
        StatusCode::CREATED,
        Json(ImportPackageResponse {
            project: imported.project,
            assets: imported.assets,
            root_revision: imported.root_revision,
            summary: ImportSummary {
                vertices: scene.vertices.len(),
                faces,
                scene_kind,
                attributes,
                original_units: units,
            },
            inventory: prepared.inventory.clone(),
        }),
    ))
}

async fn handle_list_projects(
    State(state): State<AppState>,
) -> std::result::Result<Json<Vec<Project>>, ApiErrorResponse> {
    state
        .projects
        .list_projects()
        .map(Json)
        .map_err(project_api_error)
}

async fn handle_list_assets(
    State(state): State<AppState>,
    AxumPath(project_id): AxumPath<String>,
) -> std::result::Result<Json<Vec<Asset>>, ApiErrorResponse> {
    let project_id = parse_api_uuid(&project_id, "invalid_project_id")?;
    state
        .projects
        .list_assets(project_id)
        .map(Json)
        .map_err(project_api_error)
}

async fn handle_get_project(
    State(state): State<AppState>,
    AxumPath(project_id): AxumPath<String>,
) -> std::result::Result<Json<Project>, ApiErrorResponse> {
    let project_id = parse_api_uuid(&project_id, "invalid_project_id")?;
    state
        .projects
        .load_project(project_id)
        .map(Json)
        .map_err(project_api_error)
}

async fn handle_get_asset(
    State(state): State<AppState>,
    AxumPath((project_id, asset_id)): AxumPath<(String, String)>,
) -> std::result::Result<Response, ApiErrorResponse> {
    let project_id = parse_api_uuid(&project_id, "invalid_project_id")?;
    let asset_id = parse_api_uuid(&asset_id, "invalid_asset_id")?;
    let (asset, path) = state
        .projects
        .asset_payload(project_id, asset_id)
        .map_err(project_api_error)?;
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|_| project_store_io_error())?;
    let mut response = Response::new(Body::from_stream(ReaderStream::new(file)));
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_str(&asset.media_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    response.headers_mut().insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&asset.byte_length.to_string())
            .expect("an integer is a valid header value"),
    );
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

async fn handle_export_project(
    State(state): State<AppState>,
    AxumPath(project_id): AxumPath<String>,
) -> std::result::Result<Response, ApiErrorResponse> {
    let project_id = parse_api_uuid(&project_id, "invalid_project_id")?;
    let output_path = state
        .output_dir
        .join(format!("3dmk-export-{}.zip", Uuid::new_v4()));
    let store = state.projects.clone();
    let path_for_export = output_path.clone();
    tokio::task::spawn_blocking(move || store.export_package(project_id, &path_for_export))
        .await
        .map_err(|_| project_store_io_error())?
        .map_err(project_api_error)?;
    let file = tokio::fs::File::open(&output_path)
        .await
        .map_err(|_| project_store_io_error())?;
    let length = file
        .metadata()
        .await
        .map_err(|_| project_store_io_error())?
        .len();
    let mut response = Response::new(Body::from_stream(ReaderStream::new(file)));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/zip"));
    response.headers_mut().insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&length.to_string()).expect("an integer is a valid header value"),
    );
    response.headers_mut().insert(
        "content-disposition",
        HeaderValue::from_static("attachment; filename=3dmk-project.zip"),
    );
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

async fn handle_list_revisions(
    State(state): State<AppState>,
    AxumPath(project_id): AxumPath<String>,
) -> std::result::Result<Json<Vec<Revision>>, ApiErrorResponse> {
    let project_id = parse_api_uuid(&project_id, "invalid_project_id")?;
    state
        .projects
        .list_revisions(project_id)
        .map(Json)
        .map_err(project_api_error)
}

async fn handle_set_active_revision(
    State(state): State<AppState>,
    AxumPath(project_id): AxumPath<String>,
    Json(request): Json<SetActiveRevisionRequest>,
) -> std::result::Result<Json<Project>, ApiErrorResponse> {
    let project_id = parse_api_uuid(&project_id, "invalid_project_id")?;
    let revision_id = parse_api_uuid(&request.revision_id, "invalid_revision_id")?;
    state
        .projects
        .set_active_revision(project_id, revision_id)
        .map(Json)
        .map_err(project_api_error)
}

async fn handle_get_job(
    State(state): State<AppState>,
    AxumPath(job_id): AxumPath<String>,
) -> std::result::Result<Json<JobRecord>, ApiErrorResponse> {
    let job_id = parse_api_uuid(&job_id, "invalid_job_id")?;
    state.jobs.get(job_id).map(Json).map_err(job_api_error)
}

async fn handle_cancel_job(
    State(state): State<AppState>,
    AxumPath(job_id): AxumPath<String>,
) -> std::result::Result<(StatusCode, Json<JobRecord>), ApiErrorResponse> {
    let job_id = parse_api_uuid(&job_id, "invalid_job_id")?;
    state
        .jobs
        .request_cancel(job_id)
        .map(|job| (StatusCode::ACCEPTED, Json(job)))
        .map_err(job_api_error)
}

fn parse_api_uuid(value: &str, code: &'static str) -> std::result::Result<Uuid, ApiErrorResponse> {
    Uuid::parse_str(value).map_err(|_| {
        ApiErrorResponse::new(
            StatusCode::BAD_REQUEST,
            code,
            "The supplied identifier is not a valid UUID.",
            false,
        )
    })
}

fn project_api_error(error: ProjectError) -> ApiErrorResponse {
    match error {
        ProjectError::Invalid(_) => ApiErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "invalid_project",
            "The project data is invalid.",
            false,
        ),
        ProjectError::NotFound => ApiErrorResponse::new(
            StatusCode::NOT_FOUND,
            "project_item_not_found",
            "The requested project item was not found.",
            false,
        ),
        ProjectError::Conflict(_) => ApiErrorResponse::new(
            StatusCode::CONFLICT,
            "project_conflict",
            "The requested change conflicts with the current project state.",
            false,
        ),
        ProjectError::AssetTooLarge => ApiErrorResponse::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "asset_too_large",
            "The asset exceeds the configured byte limit.",
            false,
        ),
        ProjectError::Lock => ApiErrorResponse::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "project_store_busy",
            "Project storage is temporarily unavailable.",
            true,
        ),
        ProjectError::Io(_)
        | ProjectError::Json(_)
        | ProjectError::Time(_)
        | ProjectError::Zip(_) => ApiErrorResponse::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "project_store_error",
            "Project storage could not complete the request.",
            true,
        ),
    }
}

fn job_api_error(error: JobError) -> ApiErrorResponse {
    match error {
        JobError::NotFound => ApiErrorResponse::new(
            StatusCode::NOT_FOUND,
            "job_not_found",
            "The requested job was not found.",
            false,
        ),
        JobError::QueueFull => ApiErrorResponse::new(
            StatusCode::TOO_MANY_REQUESTS,
            "job_queue_full",
            "Model processing is already at capacity. Try again after a job finishes.",
            true,
        ),
        JobError::InvalidTransition => ApiErrorResponse::new(
            StatusCode::CONFLICT,
            "invalid_job_transition",
            "The requested job action is not valid in its current state.",
            false,
        ),
        JobError::Invalid(_) => ApiErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "invalid_job_update",
            "The job update is invalid.",
            false,
        ),
        JobError::Lock => ApiErrorResponse::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "job_registry_busy",
            "Model processing status is temporarily unavailable.",
            true,
        ),
        JobError::Project(error) => project_api_error(error),
    }
}

fn package_api_error(error: PackageError) -> ApiErrorResponse {
    let status = match &error {
        PackageError::ArchiveTooLarge
        | PackageError::TooManyEntries
        | PackageError::EntryTooLarge
        | PackageError::ExpandedArchiveTooLarge
        | PackageError::CompressionRatioTooHigh => StatusCode::PAYLOAD_TOO_LARGE,
        PackageError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        PackageError::PrimaryModelChoiceRequired => StatusCode::CONFLICT,
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    };
    let retryable = matches!(&error, PackageError::Io(_));
    ApiErrorResponse::new(
        status,
        error.code(),
        "The ZIP package failed safe archive validation.",
        retryable,
    )
}

fn project_store_io_error() -> ApiErrorResponse {
    ApiErrorResponse::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        "project_store_error",
        "Project storage could not complete the request.",
        true,
    )
}

fn asset_too_large_api_error() -> ApiErrorResponse {
    ApiErrorResponse::new(
        StatusCode::PAYLOAD_TOO_LARGE,
        "asset_too_large",
        "The asset exceeds the configured byte limit.",
        false,
    )
}

fn archive_too_large_api_error() -> ApiErrorResponse {
    ApiErrorResponse::new(
        StatusCode::PAYLOAD_TOO_LARGE,
        "archive_too_large",
        "The ZIP package exceeds the configured upload limit.",
        false,
    )
}

const DIRECT_MODEL_EXTENSIONS: &[&str] = &[
    "3ds", "3mf", "dae", "fbx", "glb", "gltf", "off", "obj", "ply", "stl", "u3d", "x3d",
];

fn direct_model_extension_supported(extension: &str) -> bool {
    DIRECT_MODEL_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
}

fn model_media_type(extension: &str) -> &'static str {
    match extension {
        "3ds" => "application/x-3ds",
        "3mf" => "model/3mf",
        "dae" => "model/vnd.collada+xml",
        "fbx" => "model/fbx",
        "glb" => "model/gltf-binary",
        "gltf" => "model/gltf+json",
        "off" => "model/off",
        "obj" => "model/obj",
        "ply" => "application/ply",
        "stl" => "model/stl",
        "u3d" => "model/u3d",
        "x3d" => "model/x3d+xml",
        _ => "application/octet-stream",
    }
}

fn validated_import_units(
    extension: &str,
    requested: Option<&str>,
) -> std::result::Result<&'static str, ApiErrorResponse> {
    let requested = requested.map(str::trim).filter(|value| !value.is_empty());
    if extension == "glb" && requested.is_none() {
        return Ok("metres");
    }
    let Some(requested) = requested else {
        return Err(ApiErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "missing_units",
            "GLTF, PLY, OBJ, STL, FBX, DAE, 3DS, 3MF, OFF, U3D, X3D, and LAS imports require explicit source units.",
            false,
        ));
    };
    match requested.to_ascii_lowercase().as_str() {
        "m" | "meter" | "meters" | "metre" | "metres" => Ok("metres"),
        "ft" | "foot" | "feet" => Ok("feet"),
        "in" | "inch" | "inches" => Ok("inches"),
        "cm" | "centimeter" | "centimeters" | "centimetre" | "centimetres" => Ok("centimetres"),
        "mm" | "millimeter" | "millimeters" | "millimetre" | "millimetres" => Ok("millimetres"),
        _ => Err(ApiErrorResponse::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "unsupported_import_units",
            "Choose metres, feet, inches, centimetres, or millimetres.",
            false,
        )),
    }
}

fn canonical_scene_kind(
    scene: &vwm_core::CanonicalScene,
) -> std::result::Result<SceneKind, ApiErrorResponse> {
    match (scene.mesh, scene.point_cloud) {
        (true, true) => Ok(SceneKind::Hybrid),
        (true, false) => Ok(SceneKind::Mesh),
        (false, true) => Ok(SceneKind::PointCloud),
        (false, false) => Err(ApiErrorResponse::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "empty_scene",
            "The decoded model contains no mesh or point cloud.",
            false,
        )),
    }
}

fn canonical_scene_attributes(scene: &vwm_core::CanonicalScene) -> AttributeContract {
    let vertices = scene.vertices.len();
    AttributeContract {
        normals: scene
            .normals
            .as_ref()
            .is_some_and(|values| values.len() == vertices),
        colors: scene
            .colors
            .as_ref()
            .is_some_and(|values| values.len() == vertices),
        uvs: scene
            .uvs
            .as_ref()
            .is_some_and(|values| values.len() == vertices),
        material_ids: scene.material_ids.as_ref().is_some_and(|values| {
            scene
                .indices
                .as_ref()
                .is_some_and(|indices| values.len() == indices.len())
        }),
        source_ids: false,
        confidence: false,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CapabilityStatus {
    Available,
    Degraded,
    Experimental,
    Unavailable,
}

impl CapabilityStatus {
    fn is_usable(self) -> bool {
        self != Self::Unavailable
    }
}

#[derive(Debug, Serialize)]
struct CapabilityDependency {
    id: &'static str,
    status: CapabilityStatus,
}

#[derive(Debug, Serialize)]
struct CapabilityDescriptor {
    id: &'static str,
    status: CapabilityStatus,
    reason: &'static str,
    engine_version: Option<&'static str>,
    dependencies: Vec<CapabilityDependency>,
    supported_scene_kinds: &'static [&'static str],
    requires: &'static [&'static str],
}

#[derive(Debug, Serialize)]
struct CapabilitiesResponse {
    service: &'static str,
    version: &'static str,
    legacy_api_requests: u64,
    capabilities: Vec<CapabilityDescriptor>,
}

async fn handle_capabilities() -> Json<CapabilitiesResponse> {
    let vision = ai_vision::vision_available().await;
    let pdal = point_cloud::pdal_available();
    Json(CapabilitiesResponse {
        service: "3DMk processing service",
        version: env!("CARGO_PKG_VERSION"),
        legacy_api_requests: LEGACY_API_REQUESTS.load(Ordering::Relaxed),
        capabilities: build_capabilities(vision, pdal),
    })
}

async fn handle_health() -> Json<serde_json::Value> {
    let vision = ai_vision::vision_available().await;
    let capabilities = build_capabilities(vision, point_cloud::pdal_available());
    Json(json!({
        "status": "ok",
        "service": "3DMk processing service",
        "version": env!("CARGO_PKG_VERSION"),
        "capabilities": {
            "floorplan_to_step": true,
            "floorplan_vision": vision,
            "floorplan_local_outline": true,
            "point_cloud_to_mesh_pdal": legacy_capability_flag(&capabilities, "point_cloud_to_mesh_pdal"),
            "point_cloud_to_mesh": true,
            "point_cloud_analysis": true,
            "vwm_geometry_analysis": true,
            "vwm_implicit_poisson": true,
            "vwm_surface_nets": true,
            "vwm_perception_contracts": true,
            "vwm_perception_pipeline": true,
            "object_recognition": true,
            "flat_surface_correction": true,
            "mesh_smoothing": true,
            "image_assisted_refinement": true,
            "calibrated_photo_projection": true,
            "model_floorplan_recognition": true,
            "scene_analysis_context": true,
            "support_plane_ranking": true,
            "rust_package_ingest": true,
            "non_destructive_cleanup": false,
            "static_viewer": true
        }
    }))
}

fn build_capabilities(vision: bool, pdal: bool) -> Vec<CapabilityDescriptor> {
    let available = CapabilityStatus::Available;
    let experimental = CapabilityStatus::Experimental;
    let degraded = CapabilityStatus::Degraded;
    let unavailable = CapabilityStatus::Unavailable;

    vec![
        CapabilityDescriptor {
            id: "floorplan_to_step",
            status: available,
            reason: "Image outline extrusion is implemented and locally tested.",
            engine_version: Some(env!("CARGO_PKG_VERSION")),
            dependencies: vec![],
            supported_scene_kinds: &[],
            requires: &["floorplan_image"],
        },
        CapabilityDescriptor {
            id: "floorplan_vision",
            status: if vision { experimental } else { unavailable },
            reason: if vision {
                "A configured vision service is reachable; output still requires user review."
            } else {
                "No configured vision service is reachable."
            },
            engine_version: None,
            dependencies: vec![CapabilityDependency {
                id: "vision_service",
                status: if vision { available } else { unavailable },
            }],
            supported_scene_kinds: &[],
            requires: &["floorplan_image", "configured_vision_service"],
        },
        CapabilityDescriptor {
            id: "floorplan_local_outline",
            status: experimental,
            reason: "Local image-outline extraction exists but is not a semantic room solver.",
            engine_version: Some(env!("CARGO_PKG_VERSION")),
            dependencies: vec![],
            supported_scene_kinds: &[],
            requires: &["floorplan_image"],
        },
        CapabilityDescriptor {
            id: "point_cloud_to_mesh_pdal",
            status: if pdal { experimental } else { unavailable },
            reason: if pdal {
                "PDAL is present, but the legacy pipeline has not passed 3DMk quality gates."
            } else {
                "PDAL is not installed or on PATH; the legacy pipeline cannot run."
            },
            engine_version: None,
            dependencies: vec![CapabilityDependency {
                id: "pdal",
                status: if pdal { available } else { unavailable },
            }],
            supported_scene_kinds: &["point_cloud"],
            requires: &["oriented_normals", "quality_review"],
        },
        CapabilityDescriptor {
            id: "point_cloud_analysis",
            status: experimental,
            reason: "Deterministic component and structural analysis exists; cleanup is not revision-backed yet.",
            engine_version: Some("vwm-geometry:0.1.0"),
            dependencies: vec![CapabilityDependency {
                id: "canonical_vwm_geometry",
                status: available,
            }],
            supported_scene_kinds: &["point_cloud", "mesh"],
            requires: &[],
        },
        CapabilityDescriptor {
            id: "flat_surface_correction",
            status: degraded,
            reason: "Rigid alignment exists, but the legacy output path does not preserve the complete project context.",
            engine_version: Some(env!("CARGO_PKG_VERSION")),
            dependencies: vec![],
            supported_scene_kinds: &["point_cloud", "mesh"],
            requires: &["reviewed_support_plane"],
        },
        CapabilityDescriptor {
            id: "mesh_smoothing",
            status: degraded,
            reason: "Rust smoothing exists, but the legacy upload path is not revision-backed.",
            engine_version: Some(env!("CARGO_PKG_VERSION")),
            dependencies: vec![],
            supported_scene_kinds: &["mesh"],
            requires: &[],
        },
        CapabilityDescriptor {
            id: "image_quality_assessment",
            status: experimental,
            reason: "Sharpness, contrast, and resolution scoring are implemented.",
            engine_version: Some(env!("CARGO_PKG_VERSION")),
            dependencies: vec![],
            supported_scene_kinds: &["mesh"],
            requires: &["reference_images"],
        },
        CapabilityDescriptor {
            id: "image_assisted_refinement",
            status: experimental,
            reason: "Quality-guided contrast normalization of an existing texture image is implemented; it does not use camera calibration or alter geometry.",
            engine_version: Some(env!("CARGO_PKG_VERSION")),
            dependencies: vec![],
            supported_scene_kinds: &[],
            requires: &["package_texture", "reference_images"],
        },
        CapabilityDescriptor {
            id: "calibrated_photo_projection",
            status: experimental,
            reason: "Calibrated photos can project occlusion-aware vertex colors without changing topology. Meshes use triangle-rasterized depth and signed front-face weighting; point clouds use projected-point depth buckets and unsigned normals. Texture baking and geometry displacement are not claimed.",
            engine_version: Some(env!("CARGO_PKG_VERSION")),
            dependencies: vec![],
            supported_scene_kinds: &["mesh", "point_cloud"],
            requires: &[
                "validated_calibrated_camera_report",
                "explicit_camera_photo_bindings",
                "metres_capture_frame_provenance",
            ],
        },
        CapabilityDescriptor {
            id: "model_floorplan_recognition",
            status: experimental,
            reason: "Rust structural slice recognition is connected; review wall and corner output before export.",
            engine_version: Some(env!("CARGO_PKG_VERSION")),
            dependencies: vec![],
            supported_scene_kinds: &["mesh"],
            requires: &["three_dimensional_structural_solver"],
        },
        CapabilityDescriptor {
            id: "scene_analysis_context",
            status: experimental,
            reason: "Structural evidence is available but not stored as project analyses yet.",
            engine_version: Some("vwm-geometry:0.1.0"),
            dependencies: vec![CapabilityDependency {
                id: "canonical_vwm_geometry",
                status: available,
            }],
            supported_scene_kinds: &["point_cloud", "mesh"],
            requires: &[],
        },
        CapabilityDescriptor {
            id: "non_destructive_cleanup",
            status: unavailable,
            reason: "Transient exact component masks are implemented for review, but reviewed immutable revision publication is not yet implemented.",
            engine_version: None,
            dependencies: vec![],
            supported_scene_kinds: &["point_cloud", "mesh"],
            requires: &["project_revisions", "exact_component_masks"],
        },
        CapabilityDescriptor {
            id: "screened_poisson",
            status: experimental,
            reason: "The pure-Rust VWM Screened Poisson engine is connected as the local fallback; output still requires visual quality review.",
            engine_version: Some("vwm-implicit:0.1.0"),
            dependencies: vec![CapabilityDependency {
                id: "vwm_implicit_poisson",
                status: available,
            }],
            supported_scene_kinds: &["point_cloud"],
            requires: &["prepared_normals", "reconstruction_worker", "quality_review"],
        },
        CapabilityDescriptor {
            id: "object_recognition",
            status: experimental,
            reason: "The VWM perception endpoint is connected with deterministic region and shape fallbacks; packaged ONNX inference remains optional.",
            engine_version: Some("vwm-perception:0.1.0"),
            dependencies: vec![CapabilityDependency {
                id: "packaged_onnx_model",
                status: unavailable,
            }],
            supported_scene_kinds: &["point_cloud", "mesh"],
            requires: &["rgba_depth_id_views", "packaged_onnx_model"],
        },
        CapabilityDescriptor {
            id: "vwm_perception_pipeline",
            status: experimental,
            reason: "The canonical Rust VWM perception pipeline is connected with deterministic region and shape fallbacks; packaged ONNX inference remains optional.",
            engine_version: Some("vwm-perception:0.1.0"),
            dependencies: vec![CapabilityDependency {
                id: "canonical_vwm_perception",
                status: available,
            }],
            supported_scene_kinds: &["mesh", "point_cloud"],
            requires: &["rgba_image"],
        },
        CapabilityDescriptor {
            id: "project_revisions",
            status: experimental,
            reason: "Immutable filesystem-backed projects, source assets, imports, and revision switching are available; the existing frontend is not migrated yet.",
            engine_version: Some(env!("CARGO_PKG_VERSION")),
            dependencies: vec![CapabilityDependency {
                id: "canonical_vwm_io",
                status: available,
            }],
            supported_scene_kinds: &["point_cloud", "mesh"],
            requires: &[],
        },
        CapabilityDescriptor {
            id: "rust_package_ingest",
            status: available,
            reason: "ZIP inspection, extraction, digest verification, and project commit are Rust-owned.",
            engine_version: Some(env!("CARGO_PKG_VERSION")),
            dependencies: vec![],
            supported_scene_kinds: &[],
            requires: &["bounded_archive_parser", "project_revisions"],
        },
        CapabilityDescriptor {
            id: "static_viewer",
            status: degraded,
            reason: "The viewer works, but release assets still depend on external CDNs.",
            engine_version: Some(env!("CARGO_PKG_VERSION")),
            dependencies: vec![CapabilityDependency {
                id: "internet_or_vendored_assets",
                status: degraded,
            }],
            supported_scene_kinds: &["point_cloud", "mesh"],
            requires: &[],
        },
    ]
}

fn legacy_capability_flag(capabilities: &[CapabilityDescriptor], id: &str) -> bool {
    capabilities
        .iter()
        .find(|capability| capability.id == id)
        .is_some_and(|capability| capability.status.is_usable())
}

async fn track_legacy_api(request: Request<Body>, next: Next) -> Response {
    let legacy = is_legacy_api_path(request.uri().path());
    let mut response = next.run(request).await;
    if legacy {
        let count = LEGACY_API_REQUESTS.fetch_add(1, Ordering::Relaxed) + 1;
        response
            .headers_mut()
            .insert("deprecation", HeaderValue::from_static("true"));
        response.headers_mut().insert(
            "link",
            HeaderValue::from_static("</api/v1/capabilities>; rel=\"successor-version\""),
        );
        if let Ok(value) = HeaderValue::from_str(&count.to_string()) {
            response
                .headers_mut()
                .insert("x-3dmk-legacy-request-count", value);
        }
    }
    response
}

fn is_legacy_api_path(path: &str) -> bool {
    path.starts_with("/api/") && !path.starts_with("/api/v1/")
}

async fn handle_pdf_to_3d(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut height: f64 = 3.0;
    let mut image_data: Option<(String, Vec<u8>)> = None;

    while let Some(field) = match multipart.next_field().await {
        Ok(f) => f,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Multipart error: {}", e)})),
            );
        }
    } {
        let name = field.name().unwrap_or_default().to_string();
        if name == "image" {
            let filename = field.file_name().unwrap_or("floorplan.png").to_string();
            match field.bytes().await {
                Ok(data) => image_data = Some((filename, data.to_vec())),
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": format!("Failed to read image: {}", e)})),
                    );
                }
            }
        } else if name == "height" {
            if let Ok(text) = field.text().await {
                if let Ok(h) = text.parse::<f64>() {
                    height = h;
                }
            }
        }
    }

    let (filename, data) = match image_data {
        Some(d) => d,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing 'image' field"})),
            );
        }
    };

    let filename = safe_filename(&filename);
    if !is_floorplan_filename(&filename) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Unsupported floorplan image. Use PNG or JPG."})),
        );
    }
    if let Err(e) = std::fs::create_dir_all(&state.output_dir) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to prepare output directory: {}", e)})),
        );
    }
    let img_path = state
        .output_dir
        .join(format!("floorplan_{}_{}", chrono_simple_id(), filename));
    if let Err(e) = std::fs::write(&img_path, &data) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to save image: {}", e)})),
        );
    }

    // Extract room-aware layout through local vision or the deterministic raster fallback.
    let layout =
        match ai_vision::extract_floorplan_layout(img_path.to_str().unwrap_or_default()).await {
            Ok(layout) => layout,
            Err(e) => {
                std::fs::remove_file(&img_path).ok();
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("Floorplan extraction failed: {}", e)})),
                );
            }
        };
    std::fs::remove_file(&img_path).ok();
    let coords = layout
        .outer_boundary
        .iter()
        .map(|point| (point[0], point[1]))
        .collect::<Vec<_>>();

    // Generate STEP
    let step_name = format!("floorplan_{}.step", chrono_simple_id());
    let step_path = state.output_dir.join(&step_name);

    if let Err(e) = cad_engine::extrude_floorplan(&coords, height, &step_path) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("CAD engine failed: {}", e)})),
        );
    }

    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "step_file": format!("/output/{}", step_name),
            "vertices": coords.len(),
            "height": height,
            "layout": layout,
        })),
    )
}

async fn handle_vwm_perception(mut multipart: Multipart) -> impl IntoResponse {
    let mut image_data: Option<(String, Vec<u8>)> = None;
    let mut options = perception::VwmPerceptionOptions::default();
    let mut mode = "vlm".to_string();
    let mut vlm_options = ai_vision::VlmObjectDetectionOptions::default();
    while let Some(field) = match multipart.next_field().await {
        Ok(field) => field,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Multipart error: {error}")})),
            );
        }
    } {
        if field.name().unwrap_or_default() == "image" {
            let filename = field.file_name().unwrap_or("scene.png").to_string();
            match field.bytes().await {
                Ok(data) => image_data = Some((filename, data.to_vec())),
                Err(error) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": format!("Failed to read image: {error}")})),
                    );
                }
            }
        } else if field.name().unwrap_or_default() == "mode" {
            match field.text().await {
                Ok(value) => mode = value.trim().to_ascii_lowercase(),
                Err(error) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": format!("Failed to read perception mode: {error}")})),
                    )
                }
            }
        } else if field.name().unwrap_or_default() == "options" {
            match field.bytes().await {
                Ok(data) if data.len() <= 64 * 1024 => match serde_json::from_slice(&data) {
                    Ok(value) => options = value,
                    Err(error) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(
                                json!({"error": format!("Invalid VWM perception controls: {error}")}),
                            ),
                        )
                    }
                },
                Ok(_) => {
                    return (
                        StatusCode::PAYLOAD_TOO_LARGE,
                        Json(json!({"error": "VWM perception controls are too large"})),
                    )
                }
                Err(error) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": format!("Failed to read VWM controls: {error}")})),
                    )
                }
            }
        } else if field.name().unwrap_or_default() == "vlm_options" {
            match field.bytes().await {
                Ok(data) if data.len() <= 64 * 1024 => match serde_json::from_slice(&data) {
                    Ok(value) => vlm_options = value,
                    Err(error) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({"error": format!("Invalid VLM controls: {error}")})),
                        )
                    }
                },
                Ok(_) => {
                    return (
                        StatusCode::PAYLOAD_TOO_LARGE,
                        Json(json!({"error": "VLM controls are too large"})),
                    )
                }
                Err(error) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": format!("Failed to read VLM controls: {error}")})),
                    )
                }
            }
        }
    }
    let (filename, bytes) = match image_data {
        Some(value) => value,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing 'image' field"})),
            );
        }
    };
    if mode != "vlm" && mode != "auto" && mode != "deterministic" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Perception mode must be vlm, auto, or deterministic"})),
        );
    }
    let mut vlm_error = None;
    if mode == "vlm" || mode == "auto" {
        match ai_vision::detect_objects_vlm(&filename, &bytes, vlm_options).await {
            Ok(batch) => {
                return (
                    StatusCode::OK,
                    Json(json!({
                        "backend": "openai-compatible-vlm",
                        "mode": "vlm-object-detection",
                        "model": batch.model,
                        "image_width": batch.image_width,
                        "image_height": batch.image_height,
                        "detections": batch.detections,
                    })),
                )
            }
            Err(error) if mode == "vlm" => {
                return (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!({"error": format!("VLM object detection failed: {error}")})),
                )
            }
            Err(error) => vlm_error = Some(error.to_string()),
        }
    }
    match perception::recognize_image_with_options(&filename, &bytes, options) {
        Ok(batch) => (
            StatusCode::OK,
            Json(json!({
                "backend": "vwm-perception",
                "mode": if vlm_error.is_some() { "deterministic-color-regions-fallback" } else { "deterministic-color-regions" },
                "vlm_error": vlm_error,
                "options": options,
                "batch": batch,
            })),
        ),
        Err(error) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error": error.to_string()})),
        ),
    }
}

async fn handle_point_cloud_analysis(
    State(state): State<AppState>,
    multipart: Multipart,
) -> impl IntoResponse {
    let (input_path, _filename, metadata, floating_mesh_settings) =
        match save_ascii_cloud_upload(&state, multipart).await {
            Ok(upload) => upload,
            Err(response) => return response,
        };
    let run_input = input_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        point_cloud::analyze_file_with_metadata_and_settings(
            &run_input,
            metadata,
            floating_mesh_settings,
        )
    })
    .await;
    std::fs::remove_file(&input_path).ok();
    match result.unwrap_or_else(|error| Err(anyhow::anyhow!(error))) {
        Ok(analysis) => (
            StatusCode::OK,
            Json(json!({ "status": "ok", "analysis": analysis })),
        ),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Point-cloud analysis failed: {}", error)})),
        ),
    }
}

async fn handle_vwm_geometry_analysis(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut cloud = None::<(String, Vec<u8>)>;
    let mut options = point_cloud::VwmGeometryOptions::default();
    while let Some(field) = match multipart.next_field().await {
        Ok(field) => field,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Multipart error: {error}")})),
            )
        }
    } {
        match field.name().unwrap_or_default() {
            "cloud" => {
                let filename = safe_filename(field.file_name().unwrap_or("vwm-geometry.ply"));
                match field.bytes().await {
                    Ok(bytes) => cloud = Some((filename, bytes.to_vec())),
                    Err(error) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({"error": format!("Failed to read VWM geometry: {error}")})),
                        )
                    }
                }
            }
            "options" => match field.bytes().await {
                Ok(bytes) if bytes.len() <= 64 * 1024 => match serde_json::from_slice(&bytes) {
                    Ok(value) => options = value,
                    Err(error) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(
                                json!({"error": format!("Invalid VWM geometry controls: {error}")}),
                            ),
                        )
                    }
                },
                Ok(_) => {
                    return (
                        StatusCode::PAYLOAD_TOO_LARGE,
                        Json(json!({"error": "VWM geometry controls are too large"})),
                    )
                }
                Err(error) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(
                            json!({"error": format!("Failed to read VWM geometry controls: {error}")}),
                        ),
                    )
                }
            },
            _ => {}
        }
    }
    let (filename, bytes) = match cloud {
        Some(value) => value,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing 'cloud' field"})),
            )
        }
    };
    if !is_ply_filename(&filename) {
        return (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Json(json!({"error": "VWM geometry analysis accepts PLY meshes"})),
        );
    }
    let input_path = match save_ascii_cloud_bytes(&state, &filename, &bytes) {
        Ok(path) => path,
        Err(response) => return response,
    };
    let run_input = input_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        point_cloud::analyze_vwm_geometry_file(&run_input, options)
    })
    .await;
    std::fs::remove_file(&input_path).ok();
    match result.unwrap_or_else(|error| Err(anyhow::anyhow!(error))) {
        Ok(analysis) => (
            StatusCode::OK,
            Json(
                json!({"status": "ok", "engine": "vwm-geometry", "options": options, "analysis": analysis}),
            ),
        ),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("VWM geometry analysis failed: {error}")})),
        ),
    }
}

async fn handle_flat_surface_correction(
    State(state): State<AppState>,
    multipart: Multipart,
) -> impl IntoResponse {
    let (input_path, _filename, metadata, _floating_mesh_settings) =
        match save_ascii_cloud_upload(&state, multipart).await {
            Ok(upload) => upload,
            Err(response) => return response,
        };
    let output_name = format!("flat_corrected_{}.ply", chrono_simple_id());
    let output_path = state.output_dir.join(&output_name);
    let run_input = input_path.clone();
    let run_output = output_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        point_cloud::flatten_file_with_metadata(&run_input, &run_output, metadata)
    })
    .await;
    std::fs::remove_file(&input_path).ok();
    match result.unwrap_or_else(|error| Err(anyhow::anyhow!(error))) {
        Ok(correction) => (
            StatusCode::OK,
            Json(json!({
                "status": "ok",
                "corrected_file": format!("/output/{}", output_name),
                "correction": correction,
            })),
        ),
        Err(error) => {
            std::fs::remove_file(&output_path).ok();
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Flat-surface correction failed: {}", error)})),
            )
        }
    }
}

async fn handle_mesh_smooth(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut input_data: Option<(String, Vec<u8>)> = None;
    let mut iterations = 3usize;
    let mut strength = 0.35f64;

    while let Some(field) = match multipart.next_field().await {
        Ok(field) => field,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Multipart error: {}", error)})),
            );
        }
    } {
        match field.name().unwrap_or_default() {
            "cloud" => {
                let filename = field.file_name().unwrap_or("input.ply").to_string();
                let data = match field.bytes().await {
                    Ok(data) => data,
                    Err(error) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({"error": format!("Failed to read file: {}", error)})),
                        );
                    }
                };
                input_data = Some((filename, data.to_vec()));
            }
            "iterations" => {
                match field
                    .text()
                    .await
                    .ok()
                    .and_then(|text| text.parse::<usize>().ok())
                {
                    Some(value) => iterations = value,
                    None => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({"error": "Invalid smoothing iteration count"})),
                        );
                    }
                }
            }
            "strength" => {
                match field
                    .text()
                    .await
                    .ok()
                    .and_then(|text| text.parse::<f64>().ok())
                {
                    Some(value) => strength = value,
                    None => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({"error": "Invalid smoothing strength"})),
                        );
                    }
                }
            }
            _ => {}
        }
    }

    let (filename, data) = match input_data {
        Some(upload) => upload,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing 'cloud' field"})),
            );
        }
    };
    let filename = safe_filename(&filename);
    if !is_ascii_point_cloud_filename(&filename) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Mesh smoothing accepts ASCII PLY."})),
        );
    }
    let input_path = match save_ascii_cloud_bytes(&state, &filename, &data) {
        Ok(path) => path,
        Err(response) => return response,
    };

    let output_name = format!("smoothed_{}.ply", chrono_simple_id());
    let output_path = state.output_dir.join(&output_name);
    let run_input = input_path.clone();
    let run_output = output_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        point_cloud::smooth_file(&run_input, &run_output, iterations, strength)
    })
    .await;
    std::fs::remove_file(&input_path).ok();

    match result.unwrap_or_else(|error| Err(anyhow::anyhow!(error))) {
        Ok(smoothing) => (
            StatusCode::OK,
            Json(json!({
                "status": "ok",
                "smoothed_file": format!("/output/{}", output_name),
                "smoothing": smoothing,
            })),
        ),
        Err(error) => {
            std::fs::remove_file(&output_path).ok();
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Mesh smoothing failed: {}", error)})),
            )
        }
    }
}

async fn handle_image_assisted_refinement(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut texture: Option<image_refinement::UploadedImage> = None;
    let mut references = Vec::new();

    while let Some(field) = match multipart.next_field().await {
        Ok(field) => field,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Multipart error: {}", error)})),
            );
        }
    } {
        let field_name = field.name().unwrap_or_default().to_string();
        if field_name != "texture" && field_name != "reference_photo" {
            continue;
        }
        let filename = safe_filename(field.file_name().unwrap_or(if field_name == "texture" {
            "texture.png"
        } else {
            "reference.jpg"
        }));
        let bytes = match field.bytes().await {
            Ok(bytes) => bytes.to_vec(),
            Err(error) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": format!("Failed to read image: {}", error)})),
                );
            }
        };
        let upload = image_refinement::UploadedImage { filename, bytes };
        if field_name == "texture" {
            texture = Some(upload);
        } else {
            if references.len() >= image_refinement::MAX_REFERENCE_IMAGES {
                return (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    Json(
                        json!({"error": format!("Use at most {} reference images per refinement.", image_refinement::MAX_REFERENCE_IMAGES)}),
                    ),
                );
            }
            references.push(upload);
        }
    }

    let texture = match texture {
        Some(texture) => texture,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    json!({"error": "A package texture is required for image-assisted refinement."}),
                ),
            );
        }
    };
    let output_name = format!("refined_texture_{}.png", chrono_simple_id());
    let output_path = state.output_dir.join(&output_name);
    let refinement = tokio::task::spawn_blocking(move || {
        image_refinement::refine_texture(texture, references, &output_path)
    })
    .await;

    match refinement.unwrap_or_else(|error| Err(anyhow::anyhow!(error))) {
        Ok(refinement) => (
            StatusCode::OK,
            Json(json!({
                "status": "ok",
                "refined_texture": format!("/output/{}", output_name),
                "refinement": refinement,
            })),
        ),
        Err(error) => {
            std::fs::remove_file(state.output_dir.join(&output_name)).ok();
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Image-assisted refinement failed: {}", error)})),
            )
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CalibratedPhotoManifestEntry {
    #[serde(alias = "cameraId")]
    camera_id: String,
    #[serde(alias = "sourcePath")]
    source_path: String,
    #[serde(alias = "upload_filename")]
    filename: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CalibratedGeometryProvenance {
    units: String,
    #[serde(alias = "coordinateFrame")]
    coordinate_frame: String,
    #[serde(alias = "captureId")]
    capture_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct AsciiPlySummary {
    vertices: usize,
    faces: usize,
}

#[derive(Debug, Serialize)]
struct UnmatchedCalibratedPhoto {
    filename: String,
    camera_id: String,
    source_path: String,
    reason: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProjectionUploadLimitError {
    Field,
    Multipart,
    CompressedImages,
}

fn projection_json_error(
    status: StatusCode,
    message: impl Into<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(json!({ "error": message.into() })))
}

fn checked_projection_upload_size(
    field_bytes: usize,
    multipart_bytes: usize,
    compressed_image_bytes: usize,
    chunk_bytes: usize,
    field_limit: usize,
    is_compressed_image: bool,
) -> std::result::Result<(usize, usize, usize), ProjectionUploadLimitError> {
    let field_bytes = field_bytes
        .checked_add(chunk_bytes)
        .ok_or(ProjectionUploadLimitError::Field)?;
    if field_bytes > field_limit {
        return Err(ProjectionUploadLimitError::Field);
    }
    let multipart_bytes = multipart_bytes
        .checked_add(chunk_bytes)
        .ok_or(ProjectionUploadLimitError::Multipart)?;
    if multipart_bytes > MAX_CALIBRATED_MULTIPART_PAYLOAD_BYTES {
        return Err(ProjectionUploadLimitError::Multipart);
    }
    let compressed_image_bytes = if is_compressed_image {
        let total = compressed_image_bytes
            .checked_add(chunk_bytes)
            .ok_or(ProjectionUploadLimitError::CompressedImages)?;
        if total > MAX_CALIBRATED_COMPRESSED_IMAGE_BYTES {
            return Err(ProjectionUploadLimitError::CompressedImages);
        }
        total
    } else {
        compressed_image_bytes
    };
    Ok((field_bytes, multipart_bytes, compressed_image_bytes))
}

async fn read_calibrated_projection_field(
    mut field: Field<'_>,
    field_limit: usize,
    label: &'static str,
    multipart_bytes: &mut usize,
    compressed_image_bytes: &mut usize,
    is_compressed_image: bool,
) -> std::result::Result<Vec<u8>, (StatusCode, Json<serde_json::Value>)> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.chunk().await.map_err(|error| {
        projection_json_error(
            StatusCode::BAD_REQUEST,
            format!("Failed to read {label}: {error}"),
        )
    })? {
        let (new_field_bytes, new_multipart_bytes, new_compressed_image_bytes) =
            checked_projection_upload_size(
                bytes.len(),
                *multipart_bytes,
                *compressed_image_bytes,
                chunk.len(),
                field_limit,
                is_compressed_image,
            )
            .map_err(|limit| {
                let message = match limit {
                    ProjectionUploadLimitError::Field => format!("{label} exceeds its size limit."),
                    ProjectionUploadLimitError::Multipart => {
                        "Calibrated projection multipart payload exceeds its size limit.".to_owned()
                    }
                    ProjectionUploadLimitError::CompressedImages => {
                        "Calibrated photos exceed the aggregate compressed-image budget.".to_owned()
                    }
                };
                projection_json_error(StatusCode::PAYLOAD_TOO_LARGE, message)
            })?;
        bytes.try_reserve(chunk.len()).map_err(|_| {
            projection_json_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                format!("Could not reserve bounded memory for {label}."),
            )
        })?;
        bytes.extend_from_slice(&chunk);
        debug_assert_eq!(bytes.len(), new_field_bytes);
        *multipart_bytes = new_multipart_bytes;
        *compressed_image_bytes = new_compressed_image_bytes;
    }
    Ok(bytes)
}

fn validate_ascii_ply_input(bytes: &[u8]) -> std::result::Result<AsciiPlySummary, &'static str> {
    if bytes.is_empty() || !bytes.is_ascii() {
        return Err("Projection input must be an ASCII PLY file.");
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "Projection PLY is not valid ASCII.")?;
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("ply") {
        return Err("Projection input is missing the PLY signature.");
    }
    let mut format_seen = false;
    let mut end_header_seen = false;
    let mut vertex_count = None;
    let mut face_count = None;
    for line in lines {
        let line = line.trim();
        if line == "end_header" {
            end_header_seen = true;
            break;
        }
        if line.starts_with("format ") {
            if format_seen || line != "format ascii 1.0" {
                return Err("Projection PLY must declare exactly format ascii 1.0.");
            }
            format_seen = true;
            continue;
        }
        let Some(element) = line.strip_prefix("element ") else {
            continue;
        };
        let mut parts = element.split_whitespace();
        let name = parts.next().ok_or("Malformed PLY element declaration.")?;
        let count = parts
            .next()
            .ok_or("Malformed PLY element declaration.")?
            .parse::<usize>()
            .map_err(|_| "Malformed PLY element count.")?;
        if parts.next().is_some() {
            return Err("Malformed PLY element declaration.");
        }
        match name {
            "vertex" if vertex_count.replace(count).is_some() => {
                return Err("Projection PLY declares vertices more than once.");
            }
            "face" if face_count.replace(count).is_some() => {
                return Err("Projection PLY declares faces more than once.");
            }
            _ => {}
        }
    }
    if !format_seen || !end_header_seen {
        return Err("Projection PLY has an incomplete ASCII header.");
    }
    let vertices = vertex_count.ok_or("Projection PLY does not declare vertices.")?;
    let faces = face_count.unwrap_or(0);
    if vertices == 0 || vertices > image_refinement::MAX_PROJECTION_VERTICES {
        return Err("Projection PLY exceeds the vertex budget or is empty.");
    }
    if faces > image_refinement::MAX_PROJECTION_FACES {
        return Err("Projection PLY exceeds the face budget.");
    }
    Ok(AsciiPlySummary { vertices, faces })
}

fn validate_calibrated_provenance(
    provenance: &CalibratedGeometryProvenance,
    camera_set: &packages::IphoneCaptureReport,
) -> std::result::Result<(), &'static str> {
    if provenance.units != "metres" {
        return Err("Geometry provenance must declare units as metres.");
    }
    if provenance.coordinate_frame != "capture" {
        return Err("Geometry provenance must declare the capture coordinate frame.");
    }
    if provenance.capture_id != camera_set.capture_id {
        return Err("Geometry provenance capture_id does not match the camera report.");
    }
    Ok(())
}

fn bind_calibrated_photos(
    cameras: &[packages::CanonicalCamera],
    manifest: Vec<CalibratedPhotoManifestEntry>,
    uploads: Vec<image_refinement::UploadedImage>,
) -> std::result::Result<Vec<image_refinement::CalibratedPhoto>, Vec<UnmatchedCalibratedPhoto>> {
    let mut uploads_by_name = HashMap::with_capacity(uploads.len());
    let mut errors = Vec::new();
    for upload in uploads {
        let key = upload.filename.to_ascii_lowercase();
        match uploads_by_name.entry(key) {
            std::collections::hash_map::Entry::Occupied(_) => {
                errors.push(UnmatchedCalibratedPhoto {
                    filename: upload.filename,
                    camera_id: String::new(),
                    source_path: String::new(),
                    reason: "duplicate_upload_filename",
                });
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(upload);
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut matched_camera_ids = HashSet::with_capacity(manifest.len());
    let mut calibrated_photos = Vec::with_capacity(manifest.len());
    for entry in manifest {
        let safe_manifest_filename = safe_filename(&entry.filename);
        if safe_manifest_filename != entry.filename {
            errors.push(UnmatchedCalibratedPhoto {
                filename: entry.filename,
                camera_id: entry.camera_id,
                source_path: entry.source_path,
                reason: "unsafe_manifest_filename",
            });
            continue;
        }
        let Some(upload) = uploads_by_name.remove(&entry.filename.to_ascii_lowercase()) else {
            errors.push(UnmatchedCalibratedPhoto {
                filename: entry.filename,
                camera_id: entry.camera_id,
                source_path: entry.source_path,
                reason: "manifest_upload_not_found",
            });
            continue;
        };
        let camera = match packages::camera_for_explicit_binding(
            cameras,
            entry.camera_id.trim(),
            &entry.source_path,
        ) {
            Ok(camera) => camera.clone(),
            Err(error) => {
                errors.push(UnmatchedCalibratedPhoto {
                    filename: upload.filename,
                    camera_id: entry.camera_id,
                    source_path: entry.source_path,
                    reason: error.reason(),
                });
                continue;
            }
        };
        if !matched_camera_ids.insert(camera.id.clone()) {
            errors.push(UnmatchedCalibratedPhoto {
                filename: upload.filename,
                camera_id: entry.camera_id,
                source_path: entry.source_path,
                reason: "duplicate_camera_binding",
            });
            continue;
        }
        calibrated_photos.push(image_refinement::CalibratedPhoto {
            camera,
            image: upload,
        });
    }
    let mut unbound_uploads = uploads_by_name.into_values().collect::<Vec<_>>();
    unbound_uploads.sort_by(|left, right| left.filename.cmp(&right.filename));
    errors.extend(
        unbound_uploads
            .into_iter()
            .map(|upload| UnmatchedCalibratedPhoto {
                filename: upload.filename,
                camera_id: String::new(),
                source_path: String::new(),
                reason: "upload_missing_explicit_manifest_binding",
            }),
    );
    if errors.is_empty() && !calibrated_photos.is_empty() {
        Ok(calibrated_photos)
    } else {
        Err(errors)
    }
}

fn save_calibrated_projection_input(
    state: &AppState,
    bytes: &[u8],
) -> std::result::Result<ImportTempFile, (StatusCode, Json<serde_json::Value>)> {
    std::fs::create_dir_all(&state.output_dir).map_err(|error| {
        projection_json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to prepare projection output: {error}"),
        )
    })?;
    let input = ImportTempFile {
        path: state.output_dir.join(format!(
            "calibrated_projection_input_{}.ply",
            Uuid::new_v4()
        )),
    };
    std::fs::write(&input.path, bytes).map_err(|error| {
        projection_json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save bounded projection input: {error}"),
        )
    })?;
    Ok(input)
}

async fn handle_calibrated_photo_project(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut cloud: Option<Vec<u8>> = None;
    let mut ply_summary = None;
    let mut camera_set: Option<packages::IphoneCaptureReport> = None;
    let mut manifest: Option<Vec<CalibratedPhotoManifestEntry>> = None;
    let mut provenance: Option<CalibratedGeometryProvenance> = None;
    let mut options: Option<image_refinement::PhotoProjectionOptions> = None;
    let mut uploads = Vec::<image_refinement::UploadedImage>::new();
    let mut upload_names = HashSet::new();
    let mut multipart_bytes = 0_usize;
    let mut compressed_image_bytes = 0_usize;

    while let Some(field) = match multipart.next_field().await {
        Ok(field) => field,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Multipart error: {}", error)})),
            );
        }
    } {
        let field_name = field.name().unwrap_or_default().to_string();
        match field_name.as_str() {
            "cloud" => {
                if cloud.is_some() {
                    return projection_json_error(
                        StatusCode::BAD_REQUEST,
                        "Duplicate cloud field.",
                    );
                }
                let filename = safe_filename(field.file_name().unwrap_or("input.ply"));
                if !is_ply_filename(&filename) {
                    return projection_json_error(
                        StatusCode::UNSUPPORTED_MEDIA_TYPE,
                        "Calibrated projection accepts only PLY geometry.",
                    );
                }
                let bytes = match read_calibrated_projection_field(
                    field,
                    MAX_CALIBRATED_CLOUD_BYTES,
                    "projection cloud",
                    &mut multipart_bytes,
                    &mut compressed_image_bytes,
                    false,
                )
                .await
                {
                    Ok(bytes) => bytes,
                    Err(response) => return response,
                };
                let summary = match validate_ascii_ply_input(&bytes) {
                    Ok(summary) => summary,
                    Err(error) => {
                        return projection_json_error(StatusCode::UNPROCESSABLE_ENTITY, error)
                    }
                };
                ply_summary = Some(summary);
                cloud = Some(bytes);
            }
            "camera_set" => {
                if camera_set.is_some() {
                    return projection_json_error(
                        StatusCode::BAD_REQUEST,
                        "Duplicate camera_set field.",
                    );
                }
                let bytes = match read_calibrated_projection_field(
                    field,
                    MAX_CALIBRATED_CAMERA_REPORT_BYTES,
                    "camera report",
                    &mut multipart_bytes,
                    &mut compressed_image_bytes,
                    false,
                )
                .await
                {
                    Ok(bytes) => bytes,
                    Err(response) => return response,
                };
                match serde_json::from_slice(&bytes) {
                    Ok(value) => camera_set = Some(value),
                    Err(error) => {
                        return projection_json_error(
                            StatusCode::BAD_REQUEST,
                            format!("Invalid calibrated camera report: {error}"),
                        )
                    }
                }
            }
            "photo_manifest" => {
                if manifest.is_some() {
                    return projection_json_error(
                        StatusCode::BAD_REQUEST,
                        "Duplicate photo_manifest field.",
                    );
                }
                let bytes = match read_calibrated_projection_field(
                    field,
                    MAX_CALIBRATED_MANIFEST_BYTES,
                    "photo manifest",
                    &mut multipart_bytes,
                    &mut compressed_image_bytes,
                    false,
                )
                .await
                {
                    Ok(bytes) => bytes,
                    Err(response) => return response,
                };
                match serde_json::from_slice(&bytes) {
                    Ok(value) => manifest = Some(value),
                    Err(error) => {
                        return projection_json_error(
                            StatusCode::BAD_REQUEST,
                            format!("Invalid photo manifest: {error}"),
                        )
                    }
                }
            }
            "geometry_provenance" => {
                if provenance.is_some() {
                    return projection_json_error(
                        StatusCode::BAD_REQUEST,
                        "Duplicate geometry_provenance field.",
                    );
                }
                let bytes = match read_calibrated_projection_field(
                    field,
                    MAX_CALIBRATED_PROVENANCE_BYTES,
                    "geometry provenance",
                    &mut multipart_bytes,
                    &mut compressed_image_bytes,
                    false,
                )
                .await
                {
                    Ok(bytes) => bytes,
                    Err(response) => return response,
                };
                match serde_json::from_slice(&bytes) {
                    Ok(value) => provenance = Some(value),
                    Err(error) => {
                        return projection_json_error(
                            StatusCode::BAD_REQUEST,
                            format!("Invalid geometry provenance: {error}"),
                        )
                    }
                }
            }
            "projection_options" => {
                if options.is_some() {
                    return projection_json_error(
                        StatusCode::BAD_REQUEST,
                        "Duplicate projection_options field.",
                    );
                }
                let bytes = match read_calibrated_projection_field(
                    field,
                    MAX_CALIBRATED_OPTIONS_BYTES,
                    "projection options",
                    &mut multipart_bytes,
                    &mut compressed_image_bytes,
                    false,
                )
                .await
                {
                    Ok(bytes) => bytes,
                    Err(response) => return response,
                };
                match serde_json::from_slice(&bytes) {
                    Ok(value) => options = Some(value),
                    Err(error) => {
                        return projection_json_error(
                            StatusCode::BAD_REQUEST,
                            format!("Invalid projection options: {error}"),
                        )
                    }
                }
            }
            "reference_photo" => {
                if uploads.len() >= image_refinement::MAX_REFERENCE_IMAGES {
                    return projection_json_error(
                        StatusCode::PAYLOAD_TOO_LARGE,
                        format!(
                            "Use at most {} calibrated photos per projection.",
                            image_refinement::MAX_REFERENCE_IMAGES
                        ),
                    );
                }
                let Some(raw_filename) = field.file_name() else {
                    return projection_json_error(
                        StatusCode::BAD_REQUEST,
                        "Every reference_photo needs a filename for explicit binding.",
                    );
                };
                let filename = safe_filename(raw_filename);
                if !upload_names.insert(filename.to_ascii_lowercase()) {
                    return projection_json_error(
                        StatusCode::BAD_REQUEST,
                        format!("Duplicate calibrated photo filename: {filename}"),
                    );
                }
                let bytes = match read_calibrated_projection_field(
                    field,
                    image_refinement::MAX_IMAGE_BYTES,
                    "calibrated photo",
                    &mut multipart_bytes,
                    &mut compressed_image_bytes,
                    true,
                )
                .await
                {
                    Ok(bytes) => bytes,
                    Err(response) => return response,
                };
                uploads.push(image_refinement::UploadedImage { filename, bytes });
            }
            _ => {
                return projection_json_error(
                    StatusCode::BAD_REQUEST,
                    format!("Unknown calibrated projection field: {field_name}"),
                )
            }
        }
    }

    let cloud = match cloud {
        Some(cloud) => cloud,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "A PLY mesh or point cloud is required."})),
            );
        }
    };
    let camera_set = match camera_set {
        Some(camera_set) => camera_set,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "A calibrated iPhone camera set is required."})),
            );
        }
    };
    if let Err(error) = packages::validate_iphone_capture_report(&camera_set) {
        return projection_json_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("Invalid calibrated camera report: {error}"),
        );
    }
    if uploads.is_empty() {
        return projection_json_error(
            StatusCode::BAD_REQUEST,
            "At least one calibrated reference photo is required.",
        );
    }
    let manifest = match manifest {
        Some(manifest) if !manifest.is_empty() => manifest,
        _ => {
            return projection_json_error(
                StatusCode::BAD_REQUEST,
                "An explicit camera_id/source_path photo manifest is required.",
            )
        }
    };
    let provenance = match provenance {
        Some(provenance) => provenance,
        None => {
            return projection_json_error(
                StatusCode::BAD_REQUEST,
                "Geometry provenance declaring metres and capture frame is required.",
            )
        }
    };
    if let Err(error) = validate_calibrated_provenance(&provenance, &camera_set) {
        return projection_json_error(StatusCode::UNPROCESSABLE_ENTITY, error);
    }
    let calibrated_photos = match bind_calibrated_photos(&camera_set.cameras, manifest, uploads) {
        Ok(photos) => photos,
        Err(errors) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Every photo must bind uniquely by upload filename, camera_id, and exact package source_path.",
                    "unmatched_photos": errors,
                })),
            )
        }
    };
    let photos_matched = calibrated_photos.len();
    let input = match save_calibrated_projection_input(&state, &cloud) {
        Ok(input) => input,
        Err(response) => return response,
    };
    let output_name = format!("calibrated_vertex_colors_{}.ply", Uuid::new_v4());
    let output_path = state.output_dir.join(&output_name);
    let run_output = output_path.clone();
    let options = options.unwrap_or_default();
    let result = tokio::task::spawn_blocking(move || {
        let pending_output = PendingOutputFile::new(run_output.clone());
        match image_refinement::project_photos_to_geometry(
            &input.path,
            calibrated_photos,
            options,
            &run_output,
        ) {
            Ok(report) => {
                pending_output.preserve();
                Ok(report)
            }
            Err(error) => Err(error),
        }
    })
    .await;

    match result.unwrap_or_else(|error| Err(anyhow::anyhow!(error))) {
        Ok(projection) => (
            StatusCode::OK,
            Json(json!({
                "status": "ok",
                "operation": "occlusion_aware_vertex_color_projection",
                "projected_model": format!("/output/{}", output_name),
                "topology_preserved": true,
                "texture_baked": false,
                "geometry_displaced": false,
                "photos_matched": photos_matched,
                "input_summary": ply_summary.expect("validated PLY summary"),
                "geometry_provenance": provenance,
                "projection": projection,
                "unmatched_photos": [],
            })),
        ),
        Err(error) => {
            std::fs::remove_file(&output_path).ok();
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Calibrated photo projection failed: {}", error)})),
            )
        }
    }
}

async fn handle_model_floorplan_recognition(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut input_data: Option<(String, Vec<u8>)> = None;
    let mut scene_metadata: Option<point_cloud::ScenePackageMetadata> = None;
    let mut slice_percent = 0.4f64;

    while let Some(field) = match multipart.next_field().await {
        Ok(field) => field,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Multipart error: {}", error)})),
            );
        }
    } {
        match field.name().unwrap_or_default() {
            "cloud" => {
                let filename = field.file_name().unwrap_or("input.ply").to_string();
                let data = match field.bytes().await {
                    Ok(data) => data,
                    Err(error) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({"error": format!("Failed to read file: {}", error)})),
                        );
                    }
                };
                input_data = Some((filename, data.to_vec()));
            }
            "scene_metadata" => {
                let metadata_text = match field.text().await {
                    Ok(text) => text,
                    Err(error) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(
                                json!({"error": format!("Failed to read scene metadata: {}", error)}),
                            ),
                        );
                    }
                };
                match serde_json::from_str::<point_cloud::ScenePackageMetadata>(&metadata_text) {
                    Ok(metadata) => scene_metadata = Some(metadata),
                    Err(error) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(
                                json!({"error": format!("Invalid scene metadata JSON: {}", error)}),
                            ),
                        );
                    }
                }
            }
            "slice_percent" => {
                match field
                    .text()
                    .await
                    .ok()
                    .and_then(|text| text.parse::<f64>().ok())
                {
                    Some(value) => slice_percent = value,
                    None => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({"error": "Invalid floorplan slice percentage"})),
                        );
                    }
                }
            }
            _ => {}
        }
    }

    let (filename, data) = match input_data {
        Some(upload) => upload,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing 'cloud' field"})),
            );
        }
    };
    let filename = safe_filename(&filename);
    if !is_ascii_point_cloud_filename(&filename) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Model floorplan recognition accepts ASCII PLY."})),
        );
    }
    let input_path = match save_ascii_cloud_bytes(&state, &filename, &data) {
        Ok(path) => path,
        Err(response) => return response,
    };

    let run_input = input_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        point_cloud::recognize_floorplan_from_model(&run_input, slice_percent, scene_metadata)
    })
    .await;
    std::fs::remove_file(&input_path).ok();

    match result.unwrap_or_else(|error| Err(anyhow::anyhow!(error))) {
        Ok(recognition) => (
            StatusCode::OK,
            Json(json!({
                "status": "ok",
                "layout": recognition.layout,
                "correction": recognition.correction,
                "cut_height": recognition.cut_height,
            })),
        ),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Model floorplan recognition failed: {}", error)})),
        ),
    }
}

async fn handle_poisson(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut input_data: Option<(String, Vec<u8>)> = None;
    let mut vwm_options = point_cloud::VwmReconstructionOptions::default();

    while let Some(field) = match multipart.next_field().await {
        Ok(f) => f,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Multipart error: {}", e)})),
            );
        }
    } {
        let name = field.name().unwrap_or_default().to_string();
        if name == "cloud" {
            let filename = field.file_name().unwrap_or("input.ply").to_string();
            match field.bytes().await {
                Ok(data) => input_data = Some((filename, data.to_vec())),
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": format!("Failed to read file: {}", e)})),
                    );
                }
            }
        } else if name == "vwm_options" {
            match field.bytes().await {
                Ok(data) if data.len() <= 64 * 1024 => match serde_json::from_slice(&data) {
                    Ok(value) => vwm_options = value,
                    Err(error) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(
                                json!({"error": format!("Invalid VWM reconstruction controls: {error}")}),
                            ),
                        )
                    }
                },
                Ok(_) => {
                    return (
                        StatusCode::PAYLOAD_TOO_LARGE,
                        Json(json!({"error": "VWM reconstruction controls are too large"})),
                    )
                }
                Err(error) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": format!("Failed to read VWM controls: {error}")})),
                    )
                }
            }
        }
    }

    let (filename, data) = match input_data {
        Some(d) => d,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing 'cloud' field"})),
            );
        }
    };

    let filename = safe_filename(&filename);
    if !is_point_cloud_filename(&filename) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Unsupported point-cloud format. Use PLY, LAS, LAZ, or XYZ."})),
        );
    }

    // Save uploaded file
    if let Err(e) = std::fs::create_dir_all(&state.output_dir) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to prepare output directory: {}", e)})),
        );
    }
    let job_id = chrono_simple_id();
    let input_path = state
        .output_dir
        .join(format!("input_{}_{}", job_id, filename));
    if let Err(e) = std::fs::write(&input_path, &data) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to save file: {}", e)})),
        );
    }

    let output_name = format!("reconstructed_{}.ply", job_id);
    let output_path = state.output_dir.join(&output_name);
    let run_input = input_path.clone();
    let cleaned_input = state.output_dir.join(format!("cleaned_{}.ply", job_id));
    let run_output = output_path.clone();
    let run_cleaned = cleaned_input.clone();
    let pdal_available = point_cloud::pdal_available();
    let use_pdal = match vwm_options.backend {
        point_cloud::VwmReconstructionBackend::Auto => pdal_available,
        point_cloud::VwmReconstructionBackend::Pdal if pdal_available => true,
        point_cloud::VwmReconstructionBackend::Pdal => {
            std::fs::remove_file(&input_path).ok();
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "PDAL was selected but is not installed"})),
            );
        }
        point_cloud::VwmReconstructionBackend::Vwm => false,
    };
    if use_pdal && vwm_options.extraction == point_cloud::VwmSurfaceExtraction::SurfaceNets {
        std::fs::remove_file(&input_path).ok();
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Surface Nets requires the VWM backend"})),
        );
    }
    let result = tokio::task::spawn_blocking(move || {
        let reconstruction_input =
            match point_cloud::write_clean_reconstruction_input(&run_input, &run_cleaned) {
                Ok(()) => run_cleaned,
                Err(error) if use_pdal => {
                    // PDAL can ingest binary LAS/LAZ that the Rust artifact pass cannot parse.
                    let _ = error;
                    run_input.clone()
                }
                Err(error) => return Err(error),
            };
        if use_pdal {
            point_cloud::run_pdal_pipeline(&reconstruction_input, &run_output).map(|_| None)
        } else {
            point_cloud::run_implicit_pipeline_with_options(
                &reconstruction_input,
                &run_output,
                vwm_options,
            )
            .map(Some)
        }
    })
    .await;
    std::fs::remove_file(&input_path).ok();
    std::fs::remove_file(&cleaned_input).ok();

    let reconstruction = match result.unwrap_or_else(|e| Err(anyhow::anyhow!(e))) {
        Ok(report) => report,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Reconstruction failed: {}", e)})),
            )
        }
    };

    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "mesh_file": format!("/output/{}", output_name),
            "ply_file": format!("/output/{}", output_name),
            "backend": if use_pdal {
                "pdal"
            } else if vwm_options.extraction == point_cloud::VwmSurfaceExtraction::SurfaceNets {
                "vwm-surface-nets"
            } else {
                "vwm-screened-poisson"
            },
            "vwm": reconstruction,
        })),
    )
}

fn safe_filename(filename: &str) -> String {
    const MAX_SAFE_FILENAME_BYTES: usize = 120;
    let basename = filename
        .trim()
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default();
    let mut sanitized = String::new();
    for character in basename.chars() {
        let safe_character =
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ' ') {
                character
            } else {
                '_'
            };
        if sanitized.len() + safe_character.len_utf8() > MAX_SAFE_FILENAME_BYTES {
            break;
        }
        sanitized.push(safe_character);
    }
    let sanitized = sanitized.trim_matches(|character| character == '.' || character == ' ');
    if sanitized.is_empty() {
        "point-cloud.ply".to_owned()
    } else {
        sanitized.to_owned()
    }
}

fn is_ply_filename(filename: &str) -> bool {
    std::path::Path::new(filename)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ply"))
}

fn is_point_cloud_filename(filename: &str) -> bool {
    std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "ply" | "las" | "laz" | "xyz"
            )
        })
}

fn is_ascii_point_cloud_filename(filename: &str) -> bool {
    std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "ply" | "xyz"))
}

fn is_floorplan_filename(filename: &str) -> bool {
    std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg"))
}

async fn save_ascii_cloud_upload(
    state: &AppState,
    mut multipart: Multipart,
) -> std::result::Result<
    (
        PathBuf,
        String,
        Option<point_cloud::ScenePackageMetadata>,
        point_cloud::FloatingMeshSettings,
    ),
    (StatusCode, Json<serde_json::Value>),
> {
    let mut input_data: Option<(String, Vec<u8>)> = None;
    let mut scene_metadata: Option<point_cloud::ScenePackageMetadata> = None;
    let mut floating_mesh_settings = point_cloud::FloatingMeshSettings::default();
    while let Some(field) = multipart.next_field().await.map_err(|error| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Multipart error: {}", error)})),
        )
    })? {
        match field.name().unwrap_or_default() {
            "cloud" => {
                let filename = field.file_name().unwrap_or("input.ply").to_string();
                let data = field.bytes().await.map_err(|error| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": format!("Failed to read file: {}", error)})),
                    )
                })?;
                input_data = Some((filename, data.to_vec()));
            }
            "scene_metadata" => {
                let metadata_text = field.text().await.map_err(|error| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": format!("Failed to read scene metadata: {}", error)})),
                    )
                })?;
                let metadata = serde_json::from_str::<point_cloud::ScenePackageMetadata>(
                    &metadata_text,
                )
                .map_err(|error| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": format!("Invalid scene metadata JSON: {}", error)})),
                    )
                })?;
                scene_metadata = Some(metadata);
            }
            "floating_mesh_settings" => {
                let settings_text = field.text().await.map_err(|error| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": format!("Failed to read floating-mesh settings: {error}")})),
                    )
                })?;
                floating_mesh_settings = serde_json::from_str::<point_cloud::FloatingMeshSettings>(
                    &settings_text,
                )
                .map_err(|error| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": format!("Invalid floating-mesh settings JSON: {error}")})),
                    )
                })?
                .validate()
                .map_err(|error| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": error.to_string()})),
                    )
                })?;
            }
            _ => {}
        }
    }
    let (filename, data) = input_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Missing 'cloud' field"})),
        )
    })?;
    let filename = safe_filename(&filename);
    if !is_ascii_point_cloud_filename(&filename) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                json!({"error": "Structural analysis and flat correction accept ASCII PLY or XYZ."}),
            ),
        ));
    }
    let path = save_ascii_cloud_bytes(state, &filename, &data)?;
    Ok((path, filename, scene_metadata, floating_mesh_settings))
}

fn save_ascii_cloud_bytes(
    state: &AppState,
    filename: &str,
    data: &[u8],
) -> std::result::Result<PathBuf, (StatusCode, Json<serde_json::Value>)> {
    std::fs::create_dir_all(&state.output_dir).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to prepare output directory: {}", error)})),
        )
    })?;
    let path = state
        .output_dir
        .join(format!("input_{}_{}", chrono_simple_id(), filename));
    std::fs::write(&path, data).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to save file: {}", error)})),
        )
    })?;
    Ok(path)
}

fn chrono_simple_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}_{:06}", d.as_secs(), d.subsec_micros())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use tower::ServiceExt;
    use zip::{write::SimpleFileOptions, ZipWriter};

    struct TestApiDir(PathBuf);

    impl TestApiDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("3dmk-api-test-{}", Uuid::new_v4()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestApiDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn direct_model_import_contract_preserves_supported_formats() {
        for extension in DIRECT_MODEL_EXTENSIONS {
            assert!(
                direct_model_extension_supported(extension),
                "missing direct import support for {extension}"
            );
            assert_ne!(model_media_type(extension), "application/octet-stream");
        }
        for extension in ["exe", "html", "js", "zip"] {
            assert!(!direct_model_extension_supported(extension));
        }
        assert!(direct_model_extension_supported("GLB"));
    }

    #[test]
    fn uploaded_filename_cannot_escape_output_directory() {
        assert_eq!(safe_filename(r"..\..\cloud.ply"), "cloud.ply");
        assert_eq!(safe_filename("../../cloud.ply"), "cloud.ply");
        assert_eq!(safe_filename("bad<name>?.jpg"), "bad_name__.jpg");
        assert_eq!(safe_filename("..."), "point-cloud.ply");
    }

    #[test]
    fn point_cloud_extensions_are_checked_case_insensitively() {
        assert!(is_point_cloud_filename("scan.PLY"));
        assert!(is_point_cloud_filename("scan.laz"));
        assert!(!is_point_cloud_filename("model.glb"));
    }

    #[test]
    fn structural_tools_accept_only_ascii_formats() {
        assert!(is_ascii_point_cloud_filename("scan.PLY"));
        assert!(is_ascii_point_cloud_filename("scan.xyz"));
        assert!(!is_ascii_point_cloud_filename("scan.laz"));
        assert!(is_floorplan_filename("layout.jpeg"));
        assert!(!is_floorplan_filename("layout.pdf"));
    }

    #[test]
    fn calibrated_projection_helpers_reject_malformed_and_over_budget_input() {
        let valid = b"ply\nformat ascii 1.0\nelement vertex 1\nproperty float x\nproperty float y\nproperty float z\nend_header\n0 0 0\n";
        assert_eq!(
            validate_ascii_ply_input(valid).unwrap(),
            AsciiPlySummary {
                vertices: 1,
                faces: 0,
            }
        );
        assert!(validate_ascii_ply_input(
            b"ply\nformat binary_little_endian 1.0\nelement vertex 1\nend_header\n"
        )
        .is_err());
        let excessive_vertices = format!(
            "ply\nformat ascii 1.0\nelement vertex {}\nend_header\n",
            image_refinement::MAX_PROJECTION_VERTICES + 1
        );
        assert!(validate_ascii_ply_input(excessive_vertices.as_bytes()).is_err());

        assert_eq!(
            checked_projection_upload_size(0, 0, 0, 2, 1, false),
            Err(ProjectionUploadLimitError::Field)
        );
        assert_eq!(
            checked_projection_upload_size(
                0,
                MAX_CALIBRATED_MULTIPART_PAYLOAD_BYTES,
                0,
                1,
                2,
                false,
            ),
            Err(ProjectionUploadLimitError::Multipart)
        );
        assert_eq!(
            checked_projection_upload_size(0, 0, MAX_CALIBRATED_COMPRESSED_IMAGE_BYTES, 1, 2, true,),
            Err(ProjectionUploadLimitError::CompressedImages)
        );
    }

    fn calibrated_api_test_camera(
        id: &str,
        index: u64,
        image_path: &str,
    ) -> packages::CanonicalCamera {
        packages::CanonicalCamera {
            id: id.to_owned(),
            index,
            image_path: image_path.to_owned(),
            camera_json_path: image_path.rsplit_once('.').map_or_else(
                || format!("{image_path}.json"),
                |(stem, _)| format!("{stem}.json"),
            ),
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

    #[test]
    fn calibrated_projection_binding_uses_filename_camera_id_and_path_not_position() {
        let cameras = vec![
            calibrated_api_test_camera("camera-a", 1, "RawImages/a.jpg"),
            calibrated_api_test_camera("camera-b", 2, "RawImages/b.jpg"),
        ];
        let uploads = vec![
            image_refinement::UploadedImage {
                filename: "a.jpg".to_owned(),
                bytes: vec![1],
            },
            image_refinement::UploadedImage {
                filename: "b.jpg".to_owned(),
                bytes: vec![2],
            },
        ];
        let manifest = vec![
            CalibratedPhotoManifestEntry {
                camera_id: "camera-b".to_owned(),
                source_path: "RawImages/b.jpg".to_owned(),
                filename: "b.jpg".to_owned(),
            },
            CalibratedPhotoManifestEntry {
                camera_id: "camera-a".to_owned(),
                source_path: "RawImages/a.jpg".to_owned(),
                filename: "a.jpg".to_owned(),
            },
        ];
        let bound = bind_calibrated_photos(&cameras, manifest, uploads).unwrap();
        assert_eq!(bound[0].camera.id, "camera-b");
        assert_eq!(bound[0].image.bytes, vec![2]);
        assert_eq!(bound[1].camera.id, "camera-a");
        assert_eq!(bound[1].image.bytes, vec![1]);
    }

    #[test]
    fn capability_contract_does_not_claim_unimplemented_features() {
        let capabilities = build_capabilities(false, false);

        let cleanup = capabilities
            .iter()
            .find(|capability| capability.id == "non_destructive_cleanup")
            .expect("non-destructive cleanup capability should be reported");
        assert_eq!(cleanup.status, CapabilityStatus::Unavailable);
        assert!(cleanup.requires.contains(&"exact_component_masks"));

        for id in [
            "image_assisted_refinement",
            "calibrated_photo_projection",
            "object_recognition",
        ] {
            let capability = capabilities
                .iter()
                .find(|capability| capability.id == id)
                .unwrap_or_else(|| panic!("missing capability {id}"));
            assert_eq!(capability.status, CapabilityStatus::Experimental);
            assert!(!capability.reason.is_empty());
        }

        let calibrated_projection = capabilities
            .iter()
            .find(|capability| capability.id == "calibrated_photo_projection")
            .expect("calibrated image projection should be reported");
        assert!(calibrated_projection
            .requires
            .contains(&"explicit_camera_photo_bindings"));

        let model_floorplan = capabilities
            .iter()
            .find(|capability| capability.id == "model_floorplan_recognition")
            .expect("model floorplan recognition should be reported");
        assert_eq!(model_floorplan.status, CapabilityStatus::Experimental);
        assert_eq!(
            capabilities
                .iter()
                .find(|capability| capability.id == "screened_poisson")
                .expect("Screened Poisson capability should be reported")
                .status,
            CapabilityStatus::Experimental
        );
        assert_eq!(
            capabilities
                .iter()
                .find(|capability| capability.id == "project_revisions")
                .expect("project revision capability should be reported")
                .status,
            CapabilityStatus::Experimental
        );
        assert_eq!(
            capabilities
                .iter()
                .find(|capability| capability.id == "rust_package_ingest")
                .expect("Rust package ingestion should be reported")
                .status,
            CapabilityStatus::Available
        );
    }

    #[test]
    fn pdal_presence_never_bypasses_quality_maturity() {
        let missing = build_capabilities(false, false);
        let present = build_capabilities(false, true);

        assert!(!legacy_capability_flag(
            &missing,
            "point_cloud_to_mesh_pdal"
        ));
        let capability = present
            .iter()
            .find(|capability| capability.id == "point_cloud_to_mesh_pdal")
            .expect("PDAL capability should be reported");
        assert_eq!(capability.status, CapabilityStatus::Experimental);
        assert!(legacy_capability_flag(&present, "point_cloud_to_mesh_pdal"));
    }

    #[test]
    fn legacy_api_paths_are_identified_without_marking_v1() {
        assert!(is_legacy_api_path("/api/health"));
        assert!(is_legacy_api_path("/api/mesh-smooth"));
        assert!(!is_legacy_api_path("/api/v1/capabilities"));
        assert!(!is_legacy_api_path("/"));
    }

    #[test]
    fn imports_require_explicit_source_units_when_format_has_no_unit_contract() {
        assert_eq!(validated_import_units("glb", None).unwrap(), "metres");
        assert_eq!(validated_import_units("ply", Some("ft")).unwrap(), "feet");
        assert_eq!(
            validated_import_units("obj", None).unwrap_err().status,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(validated_import_units("ply", Some("m")).unwrap(), "metres");
        assert_eq!(
            validated_import_units("las", Some("mm")).unwrap(),
            "millimetres"
        );
    }

    #[tokio::test]
    async fn v1_import_streams_and_reopens_a_root_revision() {
        let directory = TestApiDir::new();
        let output = directory.0.join("output");
        let public = directory.0.join("public");
        let data = directory.0.join("data");
        std::fs::create_dir(&output).unwrap();
        std::fs::create_dir(&public).unwrap();
        std::fs::write(public.join("index.html"), "3DMk test").unwrap();
        let app = create_router(output.clone(), public.clone(), data.clone(), 0).unwrap();
        let boundary = "3dmk-test-boundary";
        let ply = "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n";
        let multipart = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"triangle.ply\"\r\nContent-Type: application/ply\r\n\r\n{ply}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\nTriangle\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"units\"\r\n\r\nm\r\n--{boundary}--\r\n"
        );
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/projects/import")
                    .header(
                        CONTENT_TYPE,
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(multipart))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let imported: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        let project_id = imported["project"]["project_id"].as_str().unwrap();
        let asset_id = imported["source_asset"]["asset_id"].as_str().unwrap();
        let revision_id = imported["root_revision"]["revision_id"].as_str().unwrap();
        assert_eq!(imported["summary"]["vertices"], 3);
        assert_eq!(imported["summary"]["faces"], 1);

        let revisions = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/projects/{project_id}/revisions"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(revisions.status(), StatusCode::OK);
        let revisions: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(revisions.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(revisions.as_array().unwrap().len(), 1);

        let switch = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/projects/{project_id}/active-revision"))
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "revision_id": revision_id }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(switch.status(), StatusCode::OK);

        let asset = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/projects/{project_id}/assets/{asset_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(asset.status(), StatusCode::OK);
        assert_eq!(
            axum::body::to_bytes(asset.into_body(), 1024 * 1024)
                .await
                .unwrap(),
            ply.as_bytes()
        );
        drop(app);

        let reopened = create_router(output, public, data, 0).unwrap();
        let response = reopened
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/projects/{project_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn v1_jobs_are_polled_and_cancelled_without_an_event_subscription() {
        let directory = TestApiDir::new();
        let output = directory.0.join("output");
        let public = directory.0.join("public");
        let data = directory.0.join("data");
        std::fs::create_dir(&output).unwrap();
        std::fs::create_dir(&public).unwrap();
        std::fs::write(public.join("index.html"), "3DMk test").unwrap();
        let app = create_router(output, public, data, 0).unwrap();
        let unknown_job_id = Uuid::new_v4();

        for method in ["GET", "DELETE"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(format!("/api/v1/jobs/{unknown_job_id}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/jobs/{unknown_job_id}/events"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn v1_package_inspection_streams_to_the_rust_validator() {
        let directory = TestApiDir::new();
        let output = directory.0.join("output");
        let public = directory.0.join("public");
        let data = directory.0.join("data");
        std::fs::create_dir(&output).unwrap();
        std::fs::create_dir(&public).unwrap();
        std::fs::write(public.join("index.html"), "3DMk test").unwrap();
        let app = create_router(output, public, data, 0).unwrap();

        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, contents) in [
            (
                "manifest.json",
                br#"{"geometry":{"active_asset":"geometry/model.obj"}}"#.as_slice(),
            ),
            ("geometry/model.obj", b"v 0 0 0\n".as_slice()),
            ("texture.jpg", &[0xff, 0xd8, 0xff, 0xd9]),
        ] {
            zip.start_file(name, SimpleFileOptions::default()).unwrap();
            zip.write_all(contents).unwrap();
        }
        let zip = zip.finish().unwrap().into_inner();
        let boundary = "3dmk-package-boundary";
        let mut body = Vec::new();
        write!(
            body,
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"room.zip\"\r\nContent-Type: application/zip\r\n\r\n"
        )
        .unwrap();
        body.extend_from_slice(&zip);
        write!(body, "\r\n--{boundary}--\r\n").unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/packages/inspect")
                    .header(
                        CONTENT_TYPE,
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(response["inventory"]["entry_count"], 3);
        assert_eq!(
            response["inventory"]["primary_model"]["selected_path"],
            "geometry/model.obj"
        );
    }

    #[tokio::test]
    async fn v1_package_import_commits_model_texture_reports_and_source_zip() {
        let directory = TestApiDir::new();
        let output = directory.0.join("output");
        let public = directory.0.join("public");
        let data = directory.0.join("data");
        std::fs::create_dir(&output).unwrap();
        std::fs::create_dir(&public).unwrap();
        std::fs::write(public.join("index.html"), "3DMk test").unwrap();
        let app = create_router(output, public, data, 0).unwrap();

        let obj = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
        let texture = &[0xff, 0xd8, 0xff, 0xd9];
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file("models/room.obj", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(obj).unwrap();
        zip.start_file("textures/albedo.jpg", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(texture).unwrap();
        let zip = zip.finish().unwrap().into_inner();
        let boundary = "3dmk-import-package-boundary";
        let mut body = Vec::new();
        write!(
            body,
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"room.zip\"\r\nContent-Type: application/zip\r\n\r\n"
        )
        .unwrap();
        body.extend_from_slice(&zip);
        write!(
            body,
            "\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\nRoom package\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"units\"\r\n\r\nm\r\n--{boundary}--\r\n"
        )
        .unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/projects/import-package")
                    .header(
                        CONTENT_TYPE,
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let response: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        let project_id = response["project"]["project_id"].as_str().unwrap();
        assert_eq!(response["summary"]["vertices"], 3);
        assert_eq!(response["inventory"]["file_count"], 2);
        assert!(response["assets"].as_array().unwrap().len() >= 4);

        let assets = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/projects/{project_id}/assets"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(assets.status(), StatusCode::OK);
        let assets: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(assets.into_body(), 4 * 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        assert!(assets
            .as_array()
            .unwrap()
            .iter()
            .any(|asset| asset["role"] == "source_package"));
        assert!(assets
            .as_array()
            .unwrap()
            .iter()
            .any(|asset| asset["role"] == "texture"));
    }
}
