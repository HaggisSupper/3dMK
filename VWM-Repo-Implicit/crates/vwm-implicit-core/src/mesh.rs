use crate::{ImplicitError, ImplicitResult};

pub fn compute_vertex_normals(
    positions: &[[f32; 3]],
    triangles: &[[u32; 3]],
) -> ImplicitResult<Vec<[f32; 3]>> {
    let mut normals = vec![[0.0_f64; 3]; positions.len()];
    for triangle in triangles {
        let [ia, ib, ic] = *triangle;
        let (ia, ib, ic) = (ia as usize, ib as usize, ic as usize);
        if ia >= positions.len() || ib >= positions.len() || ic >= positions.len() {
            return Err(ImplicitError::InvalidMeshIndex);
        }
        let a = positions[ia];
        let b = positions[ib];
        let c = positions[ic];
        let ab = [
            (b[0] - a[0]) as f64,
            (b[1] - a[1]) as f64,
            (b[2] - a[2]) as f64,
        ];
        let ac = [
            (c[0] - a[0]) as f64,
            (c[1] - a[1]) as f64,
            (c[2] - a[2]) as f64,
        ];
        let face = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        for index in [ia, ib, ic] {
            normals[index][0] += face[0];
            normals[index][1] += face[1];
            normals[index][2] += face[2];
        }
    }

    Ok(normals
        .into_iter()
        .map(|normal| {
            let length =
                (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            if length <= f64::EPSILON {
                [0.0, 1.0, 0.0]
            } else {
                [
                    (normal[0] / length) as f32,
                    (normal[1] / length) as f32,
                    (normal[2] / length) as f32,
                ]
            }
        })
        .collect())
}
