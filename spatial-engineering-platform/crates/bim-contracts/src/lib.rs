use spatial_types::{ArtifactId, Confidence, FrameId, Provenance};
#[derive(Debug,Clone,PartialEq,Eq)]pub enum BimClass{Site,Building,Storey,Space,Wall,Slab,Door,Window,Column,Beam,Pipe,Duct,CableTray,Equipment,Proxy}
#[derive(Debug,Clone,PartialEq,Eq)]pub enum VerificationState{Proposed,GeometricallyVerified,SemanticallyVerified,HumanApproved,Rejected}
#[derive(Debug,Clone,PartialEq)]pub struct Property{pub name:String,pub value:String,pub unit:Option<String>}
#[derive(Debug,Clone,PartialEq)]pub struct BimElement{pub id:ArtifactId,pub class:BimClass,pub name:String,pub frame:FrameId,pub source_geometry:Vec<ArtifactId>,pub properties:Vec<Property>,pub confidence:Confidence,pub verification:VerificationState,pub provenance:Provenance}
impl BimElement{pub fn exportable(&self)->bool{matches!(self.verification,VerificationState::HumanApproved|VerificationState::GeometricallyVerified|VerificationState::SemanticallyVerified)}}
