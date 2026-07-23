use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 { pub x: f64, pub y: f64, pub z: f64 }
impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    pub fn new(x: f64, y: f64, z: f64) -> Result<Self, SpatialError> {
        let v = Self { x, y, z }; v.validate()?; Ok(v)
    }
    pub fn dot(self, rhs: Self) -> f64 { self.x*rhs.x + self.y*rhs.y + self.z*rhs.z }
    pub fn cross(self, rhs: Self) -> Self { Self { x:self.y*rhs.z-self.z*rhs.y, y:self.z*rhs.x-self.x*rhs.z, z:self.x*rhs.y-self.y*rhs.x } }
    pub fn norm(self) -> f64 { self.dot(self).sqrt() }
    pub fn normalized(self) -> Result<Self, SpatialError> { let n=self.norm(); if n<=f64::EPSILON { return Err(SpatialError::Degenerate("zero-length vector")); } Ok(self.scale(1.0/n)) }
    pub fn scale(self, s:f64)->Self{Self{x:self.x*s,y:self.y*s,z:self.z*s}}
    pub fn add(self, rhs:Self)->Self{Self{x:self.x+rhs.x,y:self.y+rhs.y,z:self.z+rhs.z}}
    pub fn sub(self, rhs:Self)->Self{Self{x:self.x-rhs.x,y:self.y-rhs.y,z:self.z-rhs.z}}
    pub fn validate(self)->Result<(),SpatialError>{ if self.x.is_finite()&&self.y.is_finite()&&self.z.is_finite(){Ok(())}else{Err(SpatialError::NonFinite)} }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3(pub Vec3);
impl Point3 { pub fn new(x:f64,y:f64,z:f64)->Result<Self,SpatialError>{Ok(Self(Vec3::new(x,y,z)?))} pub fn distance(self,rhs:Self)->f64{self.0.sub(rhs.0).norm()} }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform4 { pub m:[[f64;4];4] }
impl Transform4 {
    pub const IDENTITY: Self = Self { m:[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]] };
    pub fn translation(v:Vec3)->Self{let mut t=Self::IDENTITY;t.m[0][3]=v.x;t.m[1][3]=v.y;t.m[2][3]=v.z;t}
    pub fn transform_point(self,p:Point3)->Result<Point3,SpatialError>{let v=p.0;let w=self.m[3][0]*v.x+self.m[3][1]*v.y+self.m[3][2]*v.z+self.m[3][3];if w.abs()<=f64::EPSILON{return Err(SpatialError::Degenerate("homogeneous w is zero"));}Point3::new((self.m[0][0]*v.x+self.m[0][1]*v.y+self.m[0][2]*v.z+self.m[0][3])/w,(self.m[1][0]*v.x+self.m[1][1]*v.y+self.m[1][2]*v.z+self.m[1][3])/w,(self.m[2][0]*v.x+self.m[2][1]*v.y+self.m[2][2]*v.z+self.m[2][3])/w)}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)] pub struct ArtifactId(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Hash)] pub struct FrameId(pub String);

#[derive(Debug, Clone, PartialEq)]
pub struct Provenance { pub method:String, pub implementation_version:String, pub source_artifacts:Vec<ArtifactId>, pub parameters:String }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Confidence(f64);
impl Confidence { pub fn new(value:f64)->Result<Self,SpatialError>{if value.is_finite()&&(0.0..=1.0).contains(&value){Ok(Self(value))}else{Err(SpatialError::InvalidConfidence(value))}} pub fn value(self)->f64{self.0} }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StandardUncertainty(pub f64);
impl StandardUncertainty { pub fn new(value:f64)->Result<Self,SpatialError>{if value.is_finite()&&value>=0.0{Ok(Self(value))}else{Err(SpatialError::InvalidUncertainty(value))}} }

#[derive(Debug, Clone, PartialEq)]
pub enum SpatialError { NonFinite, InvalidConfidence(f64), InvalidUncertainty(f64), Degenerate(&'static str), Parse{line:usize,message:String}, Io(String) }
impl Display for SpatialError { fn fmt(&self,f:&mut Formatter<'_>)->std::fmt::Result{match self{Self::NonFinite=>write!(f,"non-finite coordinate"),Self::InvalidConfidence(v)=>write!(f,"confidence outside [0,1]: {v}"),Self::InvalidUncertainty(v)=>write!(f,"invalid standard uncertainty: {v}"),Self::Degenerate(m)=>write!(f,"degenerate geometry: {m}"),Self::Parse{line,message}=>write!(f,"parse error on line {line}: {message}"),Self::Io(m)=>write!(f,"I/O error: {m}")}} }
impl std::error::Error for SpatialError {}
impl From<std::io::Error> for SpatialError { fn from(value:std::io::Error)->Self{Self::Io(value.to_string())} }

#[cfg(test)] mod tests { use super::*;
#[test] fn translation_moves_point(){let p=Point3::new(1.,2.,3.).unwrap();let q=Transform4::translation(Vec3::new(4.,-2.,1.).unwrap()).transform_point(p).unwrap();assert_eq!(q,Point3::new(5.,0.,4.).unwrap());}
#[test] fn confidence_rejects_invalid_values(){assert!(Confidence::new(1.01).is_err());assert_eq!(Confidence::new(0.5).unwrap().value(),0.5);}
}
