#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialFormat {
    Xyz, XyzRgb, Csv, Txt, Asc, Pts, Ptx, Ply, Obj, Stl, Off,
    E57, Las, Laz, Pcd, Gltf, Glb, Fbx, ThreeMf, U3d, Step, Iges, Usd, Usdz,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum DataKind { PointCloud, Mesh, Scene, Cad, Mixed }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum SupportLevel { Native, Partial, AdapterContract, Unsupported }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct FormatCapability { pub format: SpatialFormat, pub extension: &'static str, pub kind: DataKind, pub read: SupportLevel, pub write: SupportLevel, pub notes: &'static str }
const fn cap(format:SpatialFormat,extension:&'static str,kind:DataKind,read:SupportLevel,write:SupportLevel,notes:&'static str)->FormatCapability{FormatCapability{format,extension,kind,read,write,notes}}
pub const CAPABILITIES:&[FormatCapability]=&[
cap(SpatialFormat::Xyz,"xyz",DataKind::PointCloud,SupportLevel::Native,SupportLevel::Native,"ASCII XYZ and optional RGB"),
cap(SpatialFormat::XyzRgb,"xyzrgb",DataKind::PointCloud,SupportLevel::Native,SupportLevel::Native,"ASCII XYZRGB"),
cap(SpatialFormat::Csv,"csv",DataKind::PointCloud,SupportLevel::Native,SupportLevel::Native,"Delimited XYZ and optional RGB"),
cap(SpatialFormat::Txt,"txt",DataKind::PointCloud,SupportLevel::Native,SupportLevel::Native,"Whitespace-delimited XYZ and optional RGB"),
cap(SpatialFormat::Asc,"asc",DataKind::PointCloud,SupportLevel::Native,SupportLevel::Native,"Whitespace-delimited XYZ and optional RGB"),
cap(SpatialFormat::Pts,"pts",DataKind::PointCloud,SupportLevel::Native,SupportLevel::Native,"Count-prefixed scanner point cloud"),
cap(SpatialFormat::Ptx,"ptx",DataKind::PointCloud,SupportLevel::Native,SupportLevel::Partial,"Structured scan import; export currently flat"),
cap(SpatialFormat::Ply,"ply",DataKind::Mixed,SupportLevel::Native,SupportLevel::Native,"ASCII and binary little/big-endian read; binary little-endian write"),
cap(SpatialFormat::Obj,"obj",DataKind::Mesh,SupportLevel::Native,SupportLevel::Native,"Polygon triangulation; geometry-only"),
cap(SpatialFormat::Stl,"stl",DataKind::Mesh,SupportLevel::Native,SupportLevel::Native,"ASCII and binary read; binary write"),
cap(SpatialFormat::Off,"off",DataKind::Mesh,SupportLevel::Native,SupportLevel::Partial,"Read implemented; write pending"),
cap(SpatialFormat::E57,"e57",DataKind::PointCloud,SupportLevel::Native,SupportLevel::AdapterContract,"All point-cloud records merged; project metadata retained by future project adapter"),
cap(SpatialFormat::Las,"las",DataKind::PointCloud,SupportLevel::Native,SupportLevel::Native,"LAS 1.x point geometry and RGB"),
cap(SpatialFormat::Laz,"laz",DataKind::PointCloud,SupportLevel::Native,SupportLevel::Native,"Parallel LAZ compression/decompression"),
cap(SpatialFormat::Pcd,"pcd",DataKind::PointCloud,SupportLevel::AdapterContract,SupportLevel::AdapterContract,"PCL adapter contract"),
cap(SpatialFormat::Gltf,"gltf",DataKind::Scene,SupportLevel::Native,SupportLevel::Native,"Triangle primitives; geometry-only export"),
cap(SpatialFormat::Glb,"glb",DataKind::Scene,SupportLevel::Native,SupportLevel::Native,"Triangle primitives; geometry-only export"),
cap(SpatialFormat::Fbx,"fbx",DataKind::Scene,SupportLevel::AdapterContract,SupportLevel::AdapterContract,"External SDK adapter required"),
cap(SpatialFormat::ThreeMf,"3mf",DataKind::Mesh,SupportLevel::AdapterContract,SupportLevel::AdapterContract,"3MF adapter contract"),
cap(SpatialFormat::U3d,"u3d",DataKind::Scene,SupportLevel::AdapterContract,SupportLevel::AdapterContract,"ECMA-363/U3D codec adapter required"),
cap(SpatialFormat::Step,"step",DataKind::Cad,SupportLevel::AdapterContract,SupportLevel::AdapterContract,"OpenCascade adapter"),
cap(SpatialFormat::Iges,"iges",DataKind::Cad,SupportLevel::AdapterContract,SupportLevel::AdapterContract,"OpenCascade adapter"),
cap(SpatialFormat::Usd,"usd",DataKind::Scene,SupportLevel::AdapterContract,SupportLevel::AdapterContract,"OpenUSD adapter"),
cap(SpatialFormat::Usdz,"usdz",DataKind::Scene,SupportLevel::AdapterContract,SupportLevel::AdapterContract,"OpenUSD adapter"),
];
pub fn by_extension(extension:&str)->Option<&'static FormatCapability>{let ext=extension.trim_start_matches('.').to_ascii_lowercase();CAPABILITIES.iter().find(|c|c.extension==ext)}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_gap_formats_are_native() {
        for ext in ["e57", "las", "laz", "ply", "gltf", "glb", "stl"] {
            assert_eq!(by_extension(ext).unwrap().read, SupportLevel::Native);
        }
    }

    #[test]
    fn u3d_is_exposed_truthfully_as_an_adapter_contract() {
        let capability = by_extension("u3d").unwrap();
        assert_eq!(capability.kind, DataKind::Scene);
        assert_eq!(capability.read, SupportLevel::AdapterContract);
        assert_eq!(capability.write, SupportLevel::AdapterContract);
    }
}
