use spatial_types::{ArtifactId,Confidence,FrameId,Transform4};
#[derive(Debug,Clone,PartialEq)]pub enum EntityKind{PointSet,Mesh,Surface,Primitive,Object,Space,System}
#[derive(Debug,Clone,PartialEq)]pub struct WorldEntity{pub id:ArtifactId,pub frame:FrameId,pub kind:EntityKind,pub transform:Transform4,pub confidence:Confidence,pub source:Vec<ArtifactId>}
#[derive(Debug,Clone,PartialEq)]pub enum RelationKind{Contains,ConnectedTo,Supports,AdjacentTo,DerivedFrom,ObservedBy}
#[derive(Debug,Clone,PartialEq)]pub struct WorldRelation{pub subject:ArtifactId,pub predicate:RelationKind,pub object:ArtifactId,pub confidence:Confidence}
pub trait ObjectExtractor{type Error;fn extract(&self,source:&ArtifactId)->Result<Vec<WorldEntity>,Self::Error>;}
pub trait ClassifierBackend{type Error;fn classify(&self,entity:&WorldEntity)->Result<Vec<(String,Confidence)>,Self::Error>;}
pub trait VlmBackend{type Error;fn propose(&self,entity:&WorldEntity,image_artifacts:&[ArtifactId])->Result<Vec<(String,Confidence)>,Self::Error>;}
pub trait ComputeBackend{type Error;fn name(&self)->&str;fn device_priority(&self)->&'static [&'static str]{&["cuda","vulkan","webgpu","cpu"]}}
