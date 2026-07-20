use anyhow::{anyhow, Result};
use nalgebra::{Point3, Unit, UnitVector3, Vector3};
use rayon::prelude::*;
use vwm_core::CanonicalScene;

use crate::math::p3;

pub fn compute_face_normals(scene: &CanonicalScene) -> Result<Vec<UnitVector3<f64>>> {
    let tris = scene
        .indices
        .as_ref()
        .ok_or_else(|| anyhow!("scene has no triangle indices"))?;
    let normals: Vec<_> = tris
        .par_iter()
        .map(|tri| {
            let a = p3(scene.vertices[tri[0] as usize]);
            let b = p3(scene.vertices[tri[1] as usize]);
            let c = p3(scene.vertices[tri[2] as usize]);
            triangle_normal(a, b, c).unwrap_or_else(|| Unit::new_normalize(Vector3::y()))
        })
        .collect();
    Ok(normals)
}

pub fn triangle_normal(a: Point3<f64>, b: Point3<f64>, c: Point3<f64>) -> Option<UnitVector3<f64>> {
    let ab = b - a;
    let ac = c - a;
    Unit::try_new(ab.cross(&ac), 1e-12)
}
