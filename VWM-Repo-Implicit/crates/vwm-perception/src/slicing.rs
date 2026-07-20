use std::collections::{BTreeMap, BTreeSet};

use vwm_core::{CanonicalScene, GeometryOrigin};

use crate::{BinaryMask, ObjectSlice, PerceptionError, ProjectionMap, Result, INVALID_ELEMENT_ID};

#[derive(Debug, Clone, Copy)]
pub struct SliceConfig {
    pub minimum_visible_pixels: usize,
    pub minimum_mask_coverage: f32,
}

impl Default for SliceConfig {
    fn default() -> Self {
        Self {
            minimum_visible_pixels: 1,
            minimum_mask_coverage: 0.5,
        }
    }
}

pub fn slice_scene_by_mask(
    scene: &CanonicalScene,
    projection: &ProjectionMap,
    mask: &BinaryMask,
    config: SliceConfig,
) -> Result<ObjectSlice> {
    mask.validate()?;
    projection.validate_dimensions(mask.width, mask.height)?;
    if projection.face_ids.is_none() && projection.point_ids.is_none() {
        return Err(PerceptionError::EmptyProjection);
    }
    if !(0.0..=1.0).contains(&config.minimum_mask_coverage) {
        return Err(PerceptionError::ModelConfiguration(
            "minimum mask coverage must be in [0,1]".into(),
        ));
    }

    if let (Some(face_ids), Some(faces)) = (&projection.face_ids, &scene.indices) {
        let selected = selected_ids_by_coverage(
            face_ids,
            mask,
            config.minimum_visible_pixels,
            config.minimum_mask_coverage,
            faces.len(),
        );
        if !selected.is_empty() {
            return extract_faces(scene, &selected);
        }
    }

    if let Some(point_ids) = &projection.point_ids {
        let selected = selected_ids_by_coverage(
            point_ids,
            mask,
            config.minimum_visible_pixels,
            config.minimum_mask_coverage,
            scene.vertices.len(),
        );
        if !selected.is_empty() {
            return extract_points(scene, &selected);
        }
    }

    Err(PerceptionError::IncompatibleScene)
}

fn selected_ids_by_coverage(
    ids: &[u32],
    mask: &BinaryMask,
    minimum_visible_pixels: usize,
    minimum_mask_coverage: f32,
    upper_bound: usize,
) -> BTreeSet<u32> {
    let mut counts = BTreeMap::<u32, (usize, usize)>::new();
    for (pixel, &id) in ids.iter().enumerate() {
        if id == INVALID_ELEMENT_ID || id as usize >= upper_bound {
            continue;
        }
        let entry = counts.entry(id).or_insert((0, 0));
        entry.1 += 1;
        if mask.data[pixel] != 0 {
            entry.0 += 1;
        }
    }

    counts
        .into_iter()
        .filter_map(|(id, (selected, visible))| {
            let coverage = if visible == 0 {
                0.0
            } else {
                selected as f32 / visible as f32
            };
            (selected >= minimum_visible_pixels && coverage >= minimum_mask_coverage).then_some(id)
        })
        .collect()
}

fn extract_faces(scene: &CanonicalScene, face_ids: &BTreeSet<u32>) -> Result<ObjectSlice> {
    let faces = scene
        .indices
        .as_ref()
        .ok_or(PerceptionError::IncompatibleScene)?;
    let mut remap = BTreeMap::<u32, u32>::new();
    let mut vertices = Vec::new();
    let mut normals = scene
        .normals
        .as_ref()
        .filter(|values| values.len() == scene.vertices.len())
        .map(|_| Vec::new());
    let mut colors = scene
        .colors
        .as_ref()
        .filter(|values| values.len() == scene.vertices.len())
        .map(|_| Vec::new());
    let mut uvs = scene
        .uvs
        .as_ref()
        .filter(|values| values.len() == scene.vertices.len())
        .map(|_| Vec::new());
    let mut indices = Vec::new();
    let mut source_face_ids = Vec::new();
    let mut material_ids = scene
        .material_ids
        .as_ref()
        .filter(|values| values.len() == faces.len())
        .map(|_| Vec::new());

    for &face_id in face_ids {
        let source_face = faces
            .get(face_id as usize)
            .ok_or(PerceptionError::IncompatibleScene)?;
        let mut target_face = [0u32; 3];
        for (corner, &source_vertex) in source_face.iter().enumerate() {
            let mapped = if let Some(&existing) = remap.get(&source_vertex) {
                existing
            } else {
                let source_index = source_vertex as usize;
                let position = *scene
                    .vertices
                    .get(source_index)
                    .ok_or(PerceptionError::IncompatibleScene)?;
                let target = vertices.len() as u32;
                vertices.push(position);
                if let (Some(source), Some(target_values)) = (&scene.normals, &mut normals) {
                    if let Some(value) = source.get(source_index) {
                        target_values.push(*value);
                    }
                }
                if let (Some(source), Some(target_values)) = (&scene.colors, &mut colors) {
                    if let Some(value) = source.get(source_index) {
                        target_values.push(*value);
                    }
                }
                if let (Some(source), Some(target_values)) = (&scene.uvs, &mut uvs) {
                    if let Some(value) = source.get(source_index) {
                        target_values.push(*value);
                    }
                }
                remap.insert(source_vertex, target);
                target
            };
            target_face[corner] = mapped;
        }
        indices.push(target_face);
        source_face_ids.push(face_id);
        if let (Some(source), Some(target_values)) = (&scene.material_ids, &mut material_ids) {
            if let Some(value) = source.get(face_id as usize) {
                target_values.push(*value);
            }
        }
    }

    Ok(ObjectSlice {
        scene: CanonicalScene {
            vertices,
            normals: retain_only_complete(normals),
            indices: Some(indices),
            colors: retain_only_complete(colors),
            uvs: retain_only_complete(uvs),
            material_ids,
            mesh: true,
            point_cloud: false,
            units: scene.units,
            transform: scene.transform,
            datum: scene.datum.clone(),
            origin: GeometryOrigin::Derived,
        },
        source_face_ids,
        source_point_ids: Vec::new(),
    })
}

fn extract_points(scene: &CanonicalScene, point_ids: &BTreeSet<u32>) -> Result<ObjectSlice> {
    let mut vertices = Vec::with_capacity(point_ids.len());
    let mut normals = scene
        .normals
        .as_ref()
        .filter(|values| values.len() == scene.vertices.len())
        .map(|_| Vec::with_capacity(point_ids.len()));
    let mut colors = scene
        .colors
        .as_ref()
        .filter(|values| values.len() == scene.vertices.len())
        .map(|_| Vec::with_capacity(point_ids.len()));
    let mut uvs = scene
        .uvs
        .as_ref()
        .filter(|values| values.len() == scene.vertices.len())
        .map(|_| Vec::with_capacity(point_ids.len()));
    let mut source_point_ids = Vec::with_capacity(point_ids.len());

    for &point_id in point_ids {
        let index = point_id as usize;
        vertices.push(
            *scene
                .vertices
                .get(index)
                .ok_or(PerceptionError::IncompatibleScene)?,
        );
        if let (Some(source), Some(target)) = (&scene.normals, &mut normals) {
            if let Some(value) = source.get(index) {
                target.push(*value);
            }
        }
        if let (Some(source), Some(target)) = (&scene.colors, &mut colors) {
            if let Some(value) = source.get(index) {
                target.push(*value);
            }
        }
        if let (Some(source), Some(target)) = (&scene.uvs, &mut uvs) {
            if let Some(value) = source.get(index) {
                target.push(*value);
            }
        }
        source_point_ids.push(point_id);
    }

    Ok(ObjectSlice {
        scene: CanonicalScene {
            vertices,
            normals: retain_only_complete(normals),
            indices: None,
            colors: retain_only_complete(colors),
            uvs: retain_only_complete(uvs),
            material_ids: None,
            mesh: false,
            point_cloud: true,
            units: scene.units,
            transform: scene.transform,
            datum: scene.datum.clone(),
            origin: GeometryOrigin::Derived,
        },
        source_face_ids: Vec::new(),
        source_point_ids,
    })
}

fn retain_only_complete<T>(values: Option<Vec<T>>) -> Option<Vec<T>> {
    match values {
        Some(values) if !values.is_empty() => Some(values),
        _ => None,
    }
}
