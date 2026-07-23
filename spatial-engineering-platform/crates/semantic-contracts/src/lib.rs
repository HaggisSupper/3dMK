use spatial_types::{ArtifactId, Confidence, Provenance};
#[derive(Debug,Clone,PartialEq)]pub struct Evidence{pub kind:String,pub reference:String,pub weight:f64}
#[derive(Debug,Clone,PartialEq)]pub struct SemanticHypothesis{pub target:ArtifactId,pub class_name:String,pub confidence:Confidence,pub evidence:Vec<Evidence>,pub required_validation:Vec<String>,pub provenance:Provenance}
#[derive(Debug,Clone,PartialEq)]pub enum ValidationDecision{Accepted{reviewer:String,rationale:String},Rejected{reviewer:String,rationale:String},Deferred{reason:String}}
#[derive(Debug,Clone,PartialEq)]pub struct ValidatedSemantic{pub hypothesis:SemanticHypothesis,pub decision:ValidationDecision}
impl ValidatedSemantic{pub fn is_authoritative(&self)->bool{matches!(self.decision,ValidationDecision::Accepted{..})}}
#[cfg(test)]mod tests{use super::*;#[test]fn deferred_hypothesis_is_not_authoritative(){let h=SemanticHypothesis{target:ArtifactId("s".into()),class_name:"pump".into(),confidence:Confidence::new(0.8).unwrap(),evidence:vec![],required_validation:vec!["verify nameplate".into()],provenance:Provenance{method:"vlm".into(),implementation_version:"1".into(),source_artifacts:vec![],parameters:"{}".into()}};assert!(!ValidatedSemantic{hypothesis:h,decision:ValidationDecision::Deferred{reason:"human review".into()}}.is_authoritative());}}
