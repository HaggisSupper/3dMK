use std::path::Path;

use anyhow::{bail, Context, Result};
use gltf::{mesh::Mode, Node};
use vwm_core::{CanonicalScene, GeometryOrigin, ScenePrimitive};

type Matrix = [[f64; 4]; 4];
const IDENTITY: Matrix = [
    [1., 0., 0., 0.],
    [0., 1., 0., 0.],
    [0., 0., 1., 0.],
    [0., 0., 0., 1.],
];
fn multiply(a: Matrix, b: Matrix) -> Matrix {
    std::array::from_fn(|c| std::array::from_fn(|r| (0..4).map(|k| a[k][r] * b[c][k]).sum()))
}
fn point(m: Matrix, p: [f32; 3]) -> [f64; 3] {
    std::array::from_fn(|r| {
        m[0][r] * p[0] as f64 + m[1][r] * p[1] as f64 + m[2][r] * p[2] as f64 + m[3][r]
    })
}
fn determinant(m: Matrix) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[2][1] * m[1][2])
        - m[1][0] * (m[0][1] * m[2][2] - m[2][1] * m[0][2])
        + m[2][0] * (m[0][1] * m[1][2] - m[1][1] * m[0][2])
}
fn normal(m: Matrix, n: [f32; 3]) -> Result<[f32; 3]> {
    let det = determinant(m);
    if det.abs() < 1e-20 {
        bail!("glTF node has a singular normal transform");
    }
    let cross = |a: [f64; 4], b: [f64; 4]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let columns = [cross(m[1], m[2]), cross(m[2], m[0]), cross(m[0], m[1])];
    let v: [f64; 3] =
        std::array::from_fn(|r| (0..3).map(|c| columns[c][r] * n[c] as f64 / det).sum());
    let len = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    if !len.is_finite() || len == 0. {
        bail!("glTF contains an invalid normal");
    }
    Ok(v.map(|x| (x / len) as f32))
}

struct Builder<'a> {
    buffers: &'a [gltf::buffer::Data],
    raw_document: &'a serde_json::Value,
    scene: CanonicalScene,
    positions: Vec<[f64; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 3]>,
    faces: Vec<[u32; 3]>,
    materials: Vec<u32>,
    complete_normals: bool,
    complete_uvs: bool,
    complete_colors: bool,
}
impl Builder<'_> {
    fn visit(&mut self, node: Node<'_>, parent: Matrix, ancestry: &mut Vec<usize>) -> Result<()> {
        if ancestry.len() >= 512 || ancestry.contains(&node.index()) {
            bail!("glTF hierarchy is cyclic or deeper than 512 nodes");
        }
        ancestry.push(node.index());
        let local = raw_transform(&self.raw_document["nodes"][node.index()])?;
        let world = multiply(parent, local);
        if !world.iter().flatten().all(|v| v.is_finite()) {
            bail!("glTF node transform is not finite");
        }
        if node.skin().is_some() {
            bail!("Skinned glTF requires pose evaluation; static canonical import does not support skins");
        }
        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                if primitive.morph_targets().next().is_some() {
                    bail!("Morph-target glTF is not supported by static canonical import");
                }
                let reader = primitive.reader(|b| Some(&self.buffers[b.index()]));
                let positions: Vec<_> = reader
                    .read_positions()
                    .context("glTF primitive has no positions")?
                    .collect();
                let base = u32::try_from(self.positions.len())
                    .context("glTF exceeds u32 vertex address space")?;
                let count = positions.len();
                if count > u32::MAX as usize - base as usize {
                    bail!("glTF exceeds u32 vertex address space");
                }
                let face_start = self.faces.len();
                for p in positions {
                    let p = point(world, p);
                    if !p.iter().all(|x| x.is_finite()) {
                        bail!("glTF position is not finite");
                    }
                    self.positions.push(p);
                }
                let primitive_normals = reader
                    .read_normals()
                    .map(|values| values.map(|v| normal(world, v)).collect::<Result<Vec<_>>>())
                    .transpose()?;
                if let Some(values) = &primitive_normals {
                    self.normals.extend(values.iter().copied());
                } else {
                    self.complete_normals = false;
                }
                let mut primitive_uvs = std::collections::BTreeMap::new();
                for (semantic, _) in primitive.attributes() {
                    if let gltf::Semantic::TexCoords(set) = semantic {
                        if let Some(values) = reader.read_tex_coords(set) {
                            primitive_uvs.insert(set, values.into_f32().collect::<Vec<_>>());
                        }
                    }
                }
                if let Some(values) = primitive_uvs.get(&0) {
                    self.uvs.extend(values.iter().copied());
                } else {
                    self.complete_uvs = false;
                }
                let primitive_colors = reader
                    .read_colors(0)
                    .map(|values| values.into_rgba_f32().collect::<Vec<_>>());
                if let Some(values) = &primitive_colors {
                    self.colors
                        .extend(values.iter().map(|v| [v[0], v[1], v[2]]));
                } else {
                    self.complete_colors = false;
                }
                let raw: Vec<u32> = reader
                    .read_indices()
                    .map(|values| values.into_u32().collect())
                    .unwrap_or_else(|| (0..count as u32).collect());
                if raw.iter().any(|&i| i as usize >= count) {
                    bail!("glTF index exceeds primitive vertex count");
                }
                let mut triangles = Vec::new();
                match primitive.mode() {
                    Mode::Triangles => { if !raw.len().is_multiple_of(3) { bail!("glTF triangle index count is not divisible by three"); } triangles.extend(raw.chunks_exact(3).map(|t| [t[0],t[1],t[2]])); }
                    Mode::TriangleStrip => { for (i,t) in raw.windows(3).enumerate() { triangles.push(if i % 2 == 0 { [t[0],t[1],t[2]] } else { [t[1],t[0],t[2]] }); } }
                    Mode::TriangleFan => { for t in raw.get(1..).unwrap_or_default().windows(2) { triangles.push([raw[0],t[0],t[1]]); } }
                    Mode::Points => { self.scene.point_cloud = true; }
                    _ => bail!("glTF line primitives require an explicit line representation; they are not triangles"),
                }
                let mirrored = determinant(world) < 0.;
                let material = primitive.material().index();
                for mut t in triangles {
                    if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
                        continue;
                    }
                    if mirrored {
                        t.swap(1, 2);
                    }
                    self.faces.push(t.map(|i| i + base));
                    self.materials.push(material.map_or(u32::MAX, |i| i as u32));
                }
                self.scene.primitives.push(ScenePrimitive {
                    normals: primitive_normals,
                    texcoords: primitive_uvs,
                    colors_rgba: primitive_colors,
                    node_index: node.index(),
                    mesh_index: mesh.index(),
                    primitive_index: primitive.index(),
                    vertex_start: base as usize,
                    vertex_count: count,
                    face_start,
                    face_count: self.faces.len() - face_start,
                    material_index: material,
                    world_transform: world,
                });
            }
        }
        for child in node.children() {
            self.visit(child, world, ancestry)?;
        }
        ancestry.pop();
        Ok(())
    }
}

pub fn load_gltf(path: &Path) -> Result<CanonicalScene> {
    let original = std::fs::read(path)?;
    let raw_document: serde_json::Value = if original.starts_with(b"glTF") {
        serde_json::from_slice(&gltf::binary::Glb::from_slice(&original)?.json)?
    } else {
        serde_json::from_slice(&original)?
    };
    let (doc, buffers, _) = gltf::import(path)?;
    let mut builder = Builder {
        buffers: &buffers,
        raw_document: &raw_document,
        scene: CanonicalScene::default(),
        positions: vec![],
        normals: vec![],
        uvs: vec![],
        colors: vec![],
        faces: vec![],
        materials: vec![],
        complete_normals: true,
        complete_uvs: true,
        complete_colors: true,
    };
    let scene = doc
        .default_scene()
        .or_else(|| doc.scenes().next())
        .context("glTF has no scene")?;
    for node in scene.nodes() {
        builder.visit(node, IDENTITY, &mut vec![])?;
    }
    if builder.positions.is_empty() {
        bail!("glTF scene has no geometry");
    }
    // Rebase in f64 before narrowing, never after precision has already been lost.
    let origin = if builder
        .positions
        .iter()
        .flatten()
        .any(|v| v.abs() > 100_000.)
    {
        builder.positions[0]
    } else {
        [0.; 3]
    };
    builder.scene.vertices = builder
        .positions
        .into_iter()
        .map(|p| std::array::from_fn(|i| (p[i] - origin[i]) as f32))
        .collect();
    builder.scene.datum.origin = origin;
    builder.scene.datum.units = vwm_core::LengthUnit::Meters;
    builder.scene.units = vwm_core::LengthUnit::Meters;
    builder.scene.normals = builder.complete_normals.then_some(builder.normals);
    builder.scene.uvs = builder.complete_uvs.then_some(builder.uvs);
    builder.scene.colors = builder.complete_colors.then_some(builder.colors);
    builder.scene.mesh = !builder.faces.is_empty();
    builder.scene.indices = builder.scene.mesh.then_some(builder.faces);
    builder.scene.material_ids = builder.scene.mesh.then_some(builder.materials);
    builder.scene.origin = GeometryOrigin::Measured;
    let mut scene = builder.scene;
    scene.source_document = Some(raw_document);
    Ok(scene)
}

// gltf-json stores TRS as f32. Read transforms from original JSON to preserve
// metre-scale translations with centimetre offsets at survey coordinates.
fn raw_transform(node: &serde_json::Value) -> Result<Matrix> {
    if let Some(values) = node.get("matrix").and_then(serde_json::Value::as_array) {
        if values.len() != 16 {
            bail!("glTF matrix must contain 16 numbers");
        }
        let mut matrix = IDENTITY;
        for i in 0..16 {
            matrix[i / 4][i % 4] = values[i].as_f64().context("invalid glTF matrix value")?;
        }
        return Ok(matrix);
    }
    let array = |name: &str, default: &[f64]| -> Result<Vec<f64>> {
        match node.get(name) {
            None => Ok(default.to_vec()),
            Some(value) => {
                let values = value.as_array().context("invalid glTF transform array")?;
                if values.len() != default.len() {
                    bail!("invalid glTF transform length");
                }
                values
                    .iter()
                    .map(|v| v.as_f64().context("invalid glTF transform number"))
                    .collect()
            }
        }
    };
    let t = array("translation", &[0., 0., 0.])?;
    let q = array("rotation", &[0., 0., 0., 1.])?;
    let s = array("scale", &[1., 1., 1.])?;
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    Ok([
        [
            (1. - 2. * (y * y + z * z)) * s[0],
            2. * (x * y + z * w) * s[0],
            2. * (x * z - y * w) * s[0],
            0.,
        ],
        [
            2. * (x * y - z * w) * s[1],
            (1. - 2. * (x * x + z * z)) * s[1],
            2. * (y * z + x * w) * s[1],
            0.,
        ],
        [
            2. * (x * z + y * w) * s[2],
            2. * (y * z - x * w) * s[2],
            (1. - 2. * (x * x + y * y)) * s[2],
            0.,
        ],
        [t[0], t[1], t[2], 1.],
    ])
}
