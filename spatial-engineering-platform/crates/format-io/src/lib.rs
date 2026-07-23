use mesh_core::TriangleMesh;
use pointcloud_core::{PointCloud, PointRecord};
use spatial_types::{Point3, SpatialError};
use std::{fs, io::{Cursor, Read, Write}, path::Path};

fn external(context: &str, error: impl std::fmt::Display) -> SpatialError {
    SpatialError::Io(format!("{context}: {error}"))
}

pub fn read_point_cloud(path: &Path) -> Result<PointCloud, SpatialError> {
    match extension(path)?.as_str() {
        "e57" => read_e57(path),
        "las" | "laz" => read_las(path),
        "ply" => read_ply_points(path),
        _ => PointCloud::from_path(path),
    }
}

pub fn write_point_cloud(path: &Path, cloud: &PointCloud) -> Result<(), SpatialError> {
    match extension(path)?.as_str() {
        "las" | "laz" => write_las(path, cloud),
        "ply" => write_binary_ply_points(path, cloud),
        "xyz" | "xyzrgb" | "csv" | "txt" | "asc" | "pts" => fs::write(path, cloud.to_xyz()).map_err(Into::into),
        ext => Err(SpatialError::Parse { line: 0, message: format!("point-cloud write is not implemented for .{ext}") }),
    }
}

pub fn read_mesh(path: &Path) -> Result<TriangleMesh, SpatialError> {
    match extension(path)?.as_str() {
        "stl" => read_stl(path),
        "ply" => read_ply_mesh(path),
        "gltf" | "glb" => read_gltf(path),
        "obj" => TriangleMesh::from_obj(&fs::read_to_string(path)?),
        "off" => TriangleMesh::from_off(&fs::read_to_string(path)?),
        ext => Err(SpatialError::Parse { line: 0, message: format!("mesh read is not implemented for .{ext}") }),
    }
}

pub fn write_mesh(path: &Path, mesh: &TriangleMesh) -> Result<(), SpatialError> {
    match extension(path)?.as_str() {
        "stl" => write_binary_stl(path, mesh),
        "ply" => write_binary_ply_mesh(path, mesh),
        "glb" => write_glb(path, mesh),
        "gltf" => write_gltf(path, mesh),
        "obj" => fs::write(path, mesh.to_obj()).map_err(Into::into),
        ext => Err(SpatialError::Parse { line: 0, message: format!("mesh write is not implemented for .{ext}") }),
    }
}

fn extension(path: &Path) -> Result<String, SpatialError> {
    path.extension().and_then(|value| value.to_str()).map(str::to_ascii_lowercase)
        .ok_or_else(|| SpatialError::Parse { line: 0, message: "input has no UTF-8 extension".into() })
}

pub fn read_e57(path: &Path) -> Result<PointCloud, SpatialError> {
    let mut reader = e57::E57Reader::from_file(path).map_err(|e| external("open E57", e))?;
    let descriptors = reader.pointclouds();
    let mut points = Vec::new();
    for descriptor in descriptors {
        let iterator = reader.pointcloud_simple(&descriptor).map_err(|e| external("open E57 point cloud", e))?;
        for item in iterator {
            let point = item.map_err(|e| external("read E57 point", e))?;
            let (x, y, z) = match point.cartesian {
                e57::CartesianCoordinate::Valid { x, y, z } | e57::CartesianCoordinate::Direction { x, y, z } => (x, y, z),
                e57::CartesianCoordinate::Invalid => continue,
            };
            let rgb = point.color.map(|c| [unit_u8(c.red), unit_u8(c.green), unit_u8(c.blue)]);
            points.push(PointRecord { position: Point3::new(x, y, z)?, rgb });
        }
    }
    if points.is_empty() { return Err(SpatialError::Degenerate("E57 contains no valid Cartesian points")); }
    Ok(PointCloud { points })
}

fn unit_u8(value: f32) -> u8 { (value.clamp(0.0, 1.0) * 255.0).round() as u8 }

pub fn read_las(path: &Path) -> Result<PointCloud, SpatialError> {
    let mut reader = las::Reader::from_path(path).map_err(|e| external("open LAS/LAZ", e))?;
    let mut points = Vec::with_capacity(reader.header().number_of_points() as usize);
    for item in reader.points() {
        let point = item.map_err(|e| external("read LAS/LAZ point", e))?;
        let rgb = point.color.map(|c| [(c.red >> 8) as u8, (c.green >> 8) as u8, (c.blue >> 8) as u8]);
        points.push(PointRecord { position: Point3::new(point.x, point.y, point.z)?, rgb });
    }
    if points.is_empty() { return Err(SpatialError::Degenerate("LAS/LAZ contains no points")); }
    Ok(PointCloud { points })
}

pub fn write_las(path: &Path, cloud: &PointCloud) -> Result<(), SpatialError> {
    let has_color = cloud.points.iter().any(|p| p.rgb.is_some());
    let mut builder = las::Builder::from((1, 4));
    builder.point_format = las::point::Format::new(if has_color { 2 } else { 0 }).map_err(|e| external("create LAS format", e))?;
    builder.point_format.is_compressed = extension(path)? == "laz";
    let header = builder.into_header().map_err(|e| external("create LAS header", e))?;
    let mut writer = las::Writer::from_path(path, header).map_err(|e| external("create LAS/LAZ", e))?;
    for source in &cloud.points {
        let mut point = las::Point { x: source.position.0.x, y: source.position.0.y, z: source.position.0.z, ..Default::default() };
        point.color = source.rgb.map(|c| las::Color { red: u16::from(c[0]) * 257, green: u16::from(c[1]) * 257, blue: u16::from(c[2]) * 257 });
        writer.write_point(point).map_err(|e| external("write LAS/LAZ point", e))?;
    }
    writer.close().map_err(|e| external("finalize LAS/LAZ", e))
}

#[derive(Clone, Copy)]
enum PlyEncoding { Ascii, Little, Big }
#[derive(Clone)]
struct PlyProperty { name: String, scalar: String, list_count: Option<String> }
#[derive(Clone)]
struct PlyElement { name: String, count: usize, properties: Vec<PlyProperty> }
struct PlyHeader { encoding: PlyEncoding, elements: Vec<PlyElement>, payload_offset: usize }

pub fn read_ply_points(path: &Path) -> Result<PointCloud, SpatialError> {
    let bytes = fs::read(path)?;
    let (vertices, _) = parse_ply(&bytes)?;
    let points = vertices.into_iter().map(|(position, rgb)| PointRecord { position, rgb }).collect::<Vec<_>>();
    if points.is_empty() { return Err(SpatialError::Degenerate("PLY contains no vertices")); }
    Ok(PointCloud { points })
}

pub fn read_ply_mesh(path: &Path) -> Result<TriangleMesh, SpatialError> {
    let bytes = fs::read(path)?;
    let (vertices, triangles) = parse_ply(&bytes)?;
    if vertices.is_empty() { return Err(SpatialError::Degenerate("PLY contains no vertices")); }
    Ok(TriangleMesh { vertices: vertices.into_iter().map(|v| v.0).collect(), triangles })
}

fn parse_ply(bytes: &[u8]) -> Result<(Vec<(Point3, Option<[u8; 3]>)>, Vec<[u32; 3]>), SpatialError> {
    let header = parse_ply_header(bytes)?;
    let mut cursor = Cursor::new(&bytes[header.payload_offset..]);
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for element in header.elements {
        for _ in 0..element.count {
            if element.name == "vertex" {
                let mut xyz = [0.0; 3]; let mut rgb = [0u8; 3]; let mut has_rgb = [false; 3];
                for property in &element.properties {
                    if property.list_count.is_some() { skip_ply_list(&mut cursor, property, header.encoding)?; continue; }
                    let value = read_ply_scalar(&mut cursor, &property.scalar, header.encoding)?;
                    match property.name.as_str() {
                        "x" => xyz[0] = value, "y" => xyz[1] = value, "z" => xyz[2] = value,
                        "red" | "r" => { rgb[0] = value.round().clamp(0.0, 255.0) as u8; has_rgb[0] = true; },
                        "green" | "g" => { rgb[1] = value.round().clamp(0.0, 255.0) as u8; has_rgb[1] = true; },
                        "blue" | "b" => { rgb[2] = value.round().clamp(0.0, 255.0) as u8; has_rgb[2] = true; },
                        _ => {}
                    }
                }
                vertices.push((Point3::new(xyz[0], xyz[1], xyz[2])?, has_rgb.iter().all(|x| *x).then_some(rgb)));
            } else if element.name == "face" {
                for property in &element.properties {
                    if property.list_count.is_some() {
                        let indices = read_ply_list(&mut cursor, property, header.encoding)?;
                        if matches!(property.name.as_str(), "vertex_indices" | "vertex_index") {
                            for k in 1..indices.len().saturating_sub(1) { triangles.push([indices[0] as u32, indices[k] as u32, indices[k + 1] as u32]); }
                        }
                    } else { let _ = read_ply_scalar(&mut cursor, &property.scalar, header.encoding)?; }
                }
            } else {
                for property in &element.properties {
                    if property.list_count.is_some() { skip_ply_list(&mut cursor, property, header.encoding)?; }
                    else { let _ = read_ply_scalar(&mut cursor, &property.scalar, header.encoding)?; }
                }
            }
        }
    }
    Ok((vertices, triangles))
}

fn parse_ply_header(bytes: &[u8]) -> Result<PlyHeader, SpatialError> {
    let marker = b"end_header";
    let at = bytes.windows(marker.len()).position(|window| window == marker).ok_or_else(|| SpatialError::Parse { line: 0, message: "PLY end_header missing".into() })?;
    let mut offset = at + marker.len();
    while offset < bytes.len() && matches!(bytes[offset], b'\r' | b'\n') { offset += 1; }
    let text = std::str::from_utf8(&bytes[..at + marker.len()]).map_err(|e| external("PLY header UTF-8", e))?;
    let mut encoding = None; let mut elements = Vec::<PlyElement>::new();
    for (line_index, line) in text.lines().enumerate() {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        match parts.as_slice() {
            ["format", "ascii", "1.0"] => encoding = Some(PlyEncoding::Ascii),
            ["format", "binary_little_endian", "1.0"] => encoding = Some(PlyEncoding::Little),
            ["format", "binary_big_endian", "1.0"] => encoding = Some(PlyEncoding::Big),
            ["element", name, count] => elements.push(PlyElement { name: (*name).into(), count: count.parse().map_err(|_| SpatialError::Parse { line: line_index + 1, message: "invalid PLY element count".into() })?, properties: Vec::new() }),
            ["property", "list", count_type, item_type, name] => elements.last_mut().ok_or_else(|| SpatialError::Parse { line: line_index + 1, message: "PLY property before element".into() })?.properties.push(PlyProperty { name: (*name).into(), scalar: (*item_type).into(), list_count: Some((*count_type).into()) }),
            ["property", scalar, name] => elements.last_mut().ok_or_else(|| SpatialError::Parse { line: line_index + 1, message: "PLY property before element".into() })?.properties.push(PlyProperty { name: (*name).into(), scalar: (*scalar).into(), list_count: None }),
            _ => {}
        }
    }
    Ok(PlyHeader { encoding: encoding.ok_or_else(|| SpatialError::Parse { line: 0, message: "unsupported PLY encoding".into() })?, elements, payload_offset: offset })
}

fn read_ply_list(cursor: &mut Cursor<&[u8]>, property: &PlyProperty, encoding: PlyEncoding) -> Result<Vec<f64>, SpatialError> {
    let count_type = property.list_count.as_deref().ok_or_else(|| SpatialError::Parse { line: 0, message: "PLY property is not a list".into() })?;
    let count = read_ply_scalar(cursor, count_type, encoding)? as usize;
    (0..count).map(|_| read_ply_scalar(cursor, &property.scalar, encoding)).collect()
}
fn skip_ply_list(cursor: &mut Cursor<&[u8]>, property: &PlyProperty, encoding: PlyEncoding) -> Result<(), SpatialError> { let _ = read_ply_list(cursor, property, encoding)?; Ok(()) }
fn read_ply_scalar(cursor: &mut Cursor<&[u8]>, scalar: &str, encoding: PlyEncoding) -> Result<f64, SpatialError> {
    if matches!(encoding, PlyEncoding::Ascii) {
        let bytes = cursor.get_ref(); let mut start = cursor.position() as usize;
        while start < bytes.len() && bytes[start].is_ascii_whitespace() { start += 1; }
        let mut end = start; while end < bytes.len() && !bytes[end].is_ascii_whitespace() { end += 1; }
        let token = std::str::from_utf8(&bytes[start..end])
            .map_err(|e| external("PLY ASCII token", e))?
            .to_owned();
        cursor.set_position(end as u64);
        return token.parse().map_err(|e| external("PLY ASCII number", e));
    }
    let little = matches!(encoding, PlyEncoding::Little);
    macro_rules! bytes { ($n:expr) => {{ let mut b=[0u8;$n]; cursor.read_exact(&mut b)?; b }}; }
    Ok(match scalar {
        "char" | "int8" => i8::from_ne_bytes(bytes!(1)) as f64,
        "uchar" | "uint8" => u8::from_ne_bytes(bytes!(1)) as f64,
        "short" | "int16" => (if little { i16::from_le_bytes(bytes!(2)) } else { i16::from_be_bytes(bytes!(2)) }) as f64,
        "ushort" | "uint16" => (if little { u16::from_le_bytes(bytes!(2)) } else { u16::from_be_bytes(bytes!(2)) }) as f64,
        "int" | "int32" => (if little { i32::from_le_bytes(bytes!(4)) } else { i32::from_be_bytes(bytes!(4)) }) as f64,
        "uint" | "uint32" => (if little { u32::from_le_bytes(bytes!(4)) } else { u32::from_be_bytes(bytes!(4)) }) as f64,
        "float" | "float32" => (if little { f32::from_le_bytes(bytes!(4)) } else { f32::from_be_bytes(bytes!(4)) }) as f64,
        "double" | "float64" => if little { f64::from_le_bytes(bytes!(8)) } else { f64::from_be_bytes(bytes!(8)) },
        _ => return Err(SpatialError::Parse { line: 0, message: format!("unsupported PLY scalar type {scalar}") })
    })
}

pub fn write_binary_ply_points(path: &Path, cloud: &PointCloud) -> Result<(), SpatialError> {
    let has_color = cloud.points.iter().all(|p| p.rgb.is_some());
    let mut out = Vec::new();
    write!(out, "ply\nformat binary_little_endian 1.0\nelement vertex {}\nproperty double x\nproperty double y\nproperty double z\n", cloud.points.len())?;
    if has_color { out.extend_from_slice(b"property uchar red\nproperty uchar green\nproperty uchar blue\n"); }
    out.extend_from_slice(b"end_header\n");
    for point in &cloud.points {
        out.extend_from_slice(&point.position.0.x.to_le_bytes()); out.extend_from_slice(&point.position.0.y.to_le_bytes()); out.extend_from_slice(&point.position.0.z.to_le_bytes());
        if has_color { out.extend_from_slice(&point.rgb.unwrap()); }
    }
    fs::write(path, out)?; Ok(())
}

pub fn write_binary_ply_mesh(path: &Path, mesh: &TriangleMesh) -> Result<(), SpatialError> {
    let mut out = Vec::new();
    write!(out, "ply\nformat binary_little_endian 1.0\nelement vertex {}\nproperty double x\nproperty double y\nproperty double z\nelement face {}\nproperty list uchar uint vertex_indices\nend_header\n", mesh.vertices.len(), mesh.triangles.len())?;
    for point in &mesh.vertices { out.extend_from_slice(&point.0.x.to_le_bytes()); out.extend_from_slice(&point.0.y.to_le_bytes()); out.extend_from_slice(&point.0.z.to_le_bytes()); }
    for triangle in &mesh.triangles { out.push(3); for index in triangle { out.extend_from_slice(&index.to_le_bytes()); } }
    fs::write(path, out)?; Ok(())
}

pub fn read_stl(path: &Path) -> Result<TriangleMesh, SpatialError> {
    let bytes = fs::read(path)?;
    if bytes.starts_with(b"solid") {
        if let Ok(text) = std::str::from_utf8(&bytes) { if text.contains("facet") { return TriangleMesh::from_ascii_stl(text); } }
    }
    read_binary_stl_bytes(&bytes)
}
fn read_binary_stl_bytes(bytes: &[u8]) -> Result<TriangleMesh, SpatialError> {
    if bytes.len() < 84 { return Err(SpatialError::Parse { line: 0, message: "binary STL is shorter than 84 bytes".into() }); }
    let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
    if bytes.len() < 84 + count * 50 { return Err(SpatialError::Parse { line: 0, message: "binary STL triangle payload is truncated".into() }); }
    let mut vertices = Vec::with_capacity(count * 3); let mut triangles = Vec::with_capacity(count);
    for i in 0..count { let base = 84 + i * 50; let first = vertices.len() as u32; for vertex in 0..3 { let at = base + 12 + vertex * 12; let x=f32::from_le_bytes(bytes[at..at+4].try_into().unwrap()) as f64; let y=f32::from_le_bytes(bytes[at+4..at+8].try_into().unwrap()) as f64; let z=f32::from_le_bytes(bytes[at+8..at+12].try_into().unwrap()) as f64; vertices.push(Point3::new(x,y,z)?); } triangles.push([first,first+1,first+2]); }
    Ok(TriangleMesh { vertices, triangles })
}
pub fn write_binary_stl(path: &Path, mesh: &TriangleMesh) -> Result<(), SpatialError> {
    let mut out=vec![0u8;80]; out[..28].copy_from_slice(b"spatial-engineering-platform"); out.extend_from_slice(&(mesh.triangles.len() as u32).to_le_bytes());
    for triangle in &mesh.triangles {
        let a=mesh.vertices[triangle[0] as usize].0; let b=mesh.vertices[triangle[1] as usize].0; let c=mesh.vertices[triangle[2] as usize].0;
        let normal=b.sub(a).cross(c.sub(a)).normalized().unwrap_or(spatial_types::Vec3::ZERO);
        for value in [normal.x,normal.y,normal.z,a.x,a.y,a.z,b.x,b.y,b.z,c.x,c.y,c.z] { out.extend_from_slice(&(value as f32).to_le_bytes()); }
        out.extend_from_slice(&0u16.to_le_bytes());
    }
    fs::write(path,out)?; Ok(())
}

pub fn read_gltf(path: &Path) -> Result<TriangleMesh, SpatialError> {
    let (document, buffers, _) = gltf::import(path).map_err(|e| external("import glTF/GLB", e))?;
    let mut vertices = Vec::new(); let mut triangles = Vec::new();
    for mesh in document.meshes() { for primitive in mesh.primitives() {
        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()].0));
        let positions = reader.read_positions().ok_or_else(|| SpatialError::Parse { line: 0, message: "glTF primitive has no POSITION accessor".into() })?;
        let base = vertices.len() as u32;
        for p in positions { vertices.push(Point3::new(p[0] as f64,p[1] as f64,p[2] as f64)?); }
        let indices = reader.read_indices().map(|i| i.into_u32().collect::<Vec<_>>()).unwrap_or_else(|| (0..(vertices.len() as u32-base)).collect());
        match primitive.mode() {
            gltf::mesh::Mode::Triangles => for t in indices.chunks_exact(3) { triangles.push([base+t[0],base+t[1],base+t[2]]); },
            gltf::mesh::Mode::TriangleStrip => for i in 2..indices.len() { let (a,b)=if i%2==0{(indices[i-2],indices[i-1])}else{(indices[i-1],indices[i-2])}; triangles.push([base+a,base+b,base+indices[i]]); },
            gltf::mesh::Mode::TriangleFan => for i in 2..indices.len() { triangles.push([base+indices[0],base+indices[i-1],base+indices[i]]); },
            _ => {}
        }
    }}
    if vertices.is_empty() { return Err(SpatialError::Degenerate("glTF/GLB contains no mesh vertices")); }
    Ok(TriangleMesh { vertices, triangles })
}

pub fn write_gltf(path: &Path, mesh: &TriangleMesh) -> Result<(), SpatialError> {
    let bin_path = path.with_extension("bin"); let bin_name = bin_path.file_name().and_then(|n| n.to_str()).ok_or_else(|| SpatialError::Parse { line: 0, message: "invalid glTF buffer filename".into() })?;
    let (bin, json) = gltf_payload(mesh, Some(bin_name), false)?; fs::write(bin_path, bin)?; fs::write(path, serde_json::to_vec_pretty(&json).map_err(|e| external("serialize glTF",e))?)?; Ok(())
}
pub fn write_glb(path: &Path, mesh: &TriangleMesh) -> Result<(), SpatialError> {
    let (mut bin, json)=gltf_payload(mesh,None,true)?; while bin.len()%4!=0{bin.push(0);} let mut json=serde_json::to_vec(&json).map_err(|e| external("serialize GLB JSON",e))?; while json.len()%4!=0{json.push(b' ');} let total=12+8+json.len()+8+bin.len(); let mut out=Vec::with_capacity(total); out.extend_from_slice(b"glTF"); out.extend_from_slice(&2u32.to_le_bytes()); out.extend_from_slice(&(total as u32).to_le_bytes()); out.extend_from_slice(&(json.len() as u32).to_le_bytes()); out.extend_from_slice(&0x4E4F534Au32.to_le_bytes()); out.extend_from_slice(&json); out.extend_from_slice(&(bin.len() as u32).to_le_bytes()); out.extend_from_slice(&0x004E4942u32.to_le_bytes()); out.extend_from_slice(&bin); fs::write(path,out)?; Ok(())
}
fn gltf_payload(mesh:&TriangleMesh,uri:Option<&str>,embedded:bool)->Result<(Vec<u8>,serde_json::Value),SpatialError>{
    let mut bin=Vec::new(); for p in &mesh.vertices{for value in [p.0.x,p.0.y,p.0.z]{bin.extend_from_slice(&(value as f32).to_le_bytes());}} let index_offset=bin.len(); for t in &mesh.triangles{for i in t{bin.extend_from_slice(&i.to_le_bytes());}}
    let min=[mesh.vertices.iter().map(|p|p.0.x).fold(f64::INFINITY,f64::min),mesh.vertices.iter().map(|p|p.0.y).fold(f64::INFINITY,f64::min),mesh.vertices.iter().map(|p|p.0.z).fold(f64::INFINITY,f64::min)]; let max=[mesh.vertices.iter().map(|p|p.0.x).fold(f64::NEG_INFINITY,f64::max),mesh.vertices.iter().map(|p|p.0.y).fold(f64::NEG_INFINITY,f64::max),mesh.vertices.iter().map(|p|p.0.z).fold(f64::NEG_INFINITY,f64::max)];
    let mut buffer=serde_json::json!({"byteLength":bin.len()}); if !embedded{buffer["uri"]=serde_json::Value::String(uri.unwrap().into());}
    let json=serde_json::json!({"asset":{"version":"2.0","generator":"spatial-engineering-platform"},"buffers":[buffer],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":index_offset,"target":34962},{"buffer":0,"byteOffset":index_offset,"byteLength":bin.len()-index_offset,"target":34963}],"accessors":[{"bufferView":0,"componentType":5126,"count":mesh.vertices.len(),"type":"VEC3","min":min,"max":max},{"bufferView":1,"componentType":5125,"count":mesh.triangles.len()*3,"type":"SCALAR"}],"meshes":[{"primitives":[{"attributes":{"POSITION":0},"indices":1,"mode":4}]}],"nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0});
    Ok((bin,json))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn triangle() -> TriangleMesh { TriangleMesh { vertices: vec![Point3::new(0.,0.,0.).unwrap(),Point3::new(1.,0.,0.).unwrap(),Point3::new(0.,1.,0.).unwrap()], triangles: vec![[0,1,2]] } }
    #[test] fn binary_stl_roundtrip() { let mesh=triangle(); let mut path=std::env::temp_dir();path.push("sep-roundtrip.stl");write_binary_stl(&path,&mesh).unwrap();let restored=read_stl(&path).unwrap();assert_eq!(restored.triangles.len(),1);let _=fs::remove_file(path); }
    #[test] fn binary_ply_mesh_roundtrip() { let mesh=triangle();let mut path=std::env::temp_dir();path.push("sep-roundtrip.ply");write_binary_ply_mesh(&path,&mesh).unwrap();let restored=read_ply_mesh(&path).unwrap();assert_eq!(restored.triangles,vec![[0,1,2]]);let _=fs::remove_file(path); }
    #[test] fn glb_has_valid_header() { let mesh=triangle();let mut path=std::env::temp_dir();path.push("sep-roundtrip.glb");write_glb(&path,&mesh).unwrap();let bytes=fs::read(&path).unwrap();assert_eq!(&bytes[..4],b"glTF");let _=fs::remove_file(path); }
}
