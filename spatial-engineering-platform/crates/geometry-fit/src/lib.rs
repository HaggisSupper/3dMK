use pointcloud_core::PointCloud;
use spatial_types::{Confidence, Point3, SpatialError, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FitQuality { pub support_count:usize, pub rms:f64, pub max_abs:f64, pub confidence:Confidence }
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaneFit { pub origin:Point3, pub normal:Vec3, pub quality:FitQuality }
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrincipalAxis { X, Y, Z }
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CylinderFit { pub axis:PrincipalAxis, pub center_a:f64, pub center_b:f64, pub radius:f64, pub quality:FitQuality }

pub fn fit_plane(cloud:&PointCloud)->Result<PlaneFit,SpatialError>{
    if cloud.points.len()<3{return Err(SpatialError::Degenerate("plane fit requires at least three points"));}
    let centroid=cloud.summary()?.centroid;
    let c=centroid.0;let mut xx=0.;let mut xy=0.;let mut xz=0.;let mut yy=0.;let mut yz=0.;let mut zz=0.;
    for p in &cloud.points{let d=p.position.0.sub(c);xx+=d.x*d.x;xy+=d.x*d.y;xz+=d.x*d.z;yy+=d.y*d.y;yz+=d.y*d.z;zz+=d.z*d.z;}
    let det_x=yy*zz-yz*yz;let det_y=xx*zz-xz*xz;let det_z=xx*yy-xy*xy;
    let normal=if det_x>=det_y&&det_x>=det_z{Vec3{x:det_x,y:xz*yz-xy*zz,z:xy*yz-xz*yy}}else if det_y>=det_z{Vec3{x:xz*yz-xy*zz,y:det_y,z:xy*xz-yz*xx}}else{Vec3{x:xy*yz-xz*yy,y:xy*xz-yz*xx,z:det_z}}.normalized()?;
    let mut sum_sq=0.;let mut max_abs:f64=0.;for p in &cloud.points{let r=p.position.0.sub(c).dot(normal).abs();sum_sq+=r*r;max_abs=max_abs.max(r);}let rms=(sum_sq/cloud.points.len() as f64).sqrt();
    let span=cloud.summary()?;let diagonal=span.max.0.sub(span.min.0).norm().max(f64::EPSILON);let confidence=Confidence::new((1.0-rms/diagonal).clamp(0.0,1.0))?;
    Ok(PlaneFit{origin:centroid,normal,quality:FitQuality{support_count:cloud.points.len(),rms,max_abs,confidence}})
}

pub fn fit_axis_aligned_cylinder(cloud:&PointCloud,axis:PrincipalAxis)->Result<CylinderFit,SpatialError>{
    if cloud.points.len()<3{return Err(SpatialError::Degenerate("cylinder fit requires at least three points"));}
    let projected:Vec<(f64,f64)>=cloud.points.iter().map(|p|{let v=p.position.0;match axis{PrincipalAxis::X=>(v.y,v.z),PrincipalAxis::Y=>(v.x,v.z),PrincipalAxis::Z=>(v.x,v.y)}}).collect();
    let n=projected.len() as f64;let ca=projected.iter().map(|v|v.0).sum::<f64>()/n;let cb=projected.iter().map(|v|v.1).sum::<f64>()/n;let radius=projected.iter().map(|(a,b)|((a-ca).powi(2)+(b-cb).powi(2)).sqrt()).sum::<f64>()/n;
    if radius<=f64::EPSILON{return Err(SpatialError::Degenerate("cylinder radius is zero"));}
    let residuals:Vec<f64>=projected.iter().map(|(a,b)|(((a-ca).powi(2)+(b-cb).powi(2)).sqrt()-radius).abs()).collect();let rms=(residuals.iter().map(|r|r*r).sum::<f64>()/n).sqrt();let max_abs=residuals.iter().copied().fold(0.0,f64::max);let confidence=Confidence::new((1.0-rms/radius).clamp(0.0,1.0))?;
    Ok(CylinderFit{axis,center_a:ca,center_b:cb,radius,quality:FitQuality{support_count:cloud.points.len(),rms,max_abs,confidence}})
}

#[cfg(test)] mod tests{use super::*;use pointcloud_core::PointCloud;
#[test]fn plane_fit_finds_horizontal_plane(){let c=PointCloud::from_xyz_text("0 0 2\n1 0 2\n0 1 2\n1 1 2\n").unwrap();let f=fit_plane(&c).unwrap();assert!(f.normal.z.abs()>0.999999);assert!(f.quality.rms<1e-12);}
#[test]fn cylinder_fit_finds_radius(){let c=PointCloud::from_xyz_text("1 0 0\n0 1 0\n-1 0 0\n0 -1 0\n").unwrap();let f=fit_axis_aligned_cylinder(&c,PrincipalAxis::Z).unwrap();assert!((f.radius-1.0).abs()<1e-12);}
}
