//! Deterministic, typed admission for the 3DMK local-intelligence ladder.
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

pub const LOCAL_INTELLIGENCE_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_REQUEST_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum IntelligenceOperation {
    MetadataInspection,
    GeometryValidation,
    EvidenceSummarization,
    VisionAdjudication,
    FloorplanInterpretation,
    GeneralAssistance,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntelligenceTier {
    Deterministic,
    MistralRs,
    LlamaCpp,
    OpenAiCompatibleCloud,
    Blocked,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntelligenceBlockReason {
    IncompatibleContractVersion,
    RequestTooLarge,
    UnsupportedOperation,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IntelligenceRequest {
    pub contract_version: u16,
    pub operation: IntelligenceOperation,
    pub input_bytes: u64,
    pub requires_authoritative_state: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IntelligenceDecision {
    pub contract_version: u16,
    pub operation: IntelligenceOperation,
    pub tier: IntelligenceTier,
    pub advisory_only: bool,
    pub automatic_fallback_allowed: bool,
    pub block_reason: Option<IntelligenceBlockReason>,
}

impl IntelligenceDecision {
    const fn blocked(
        operation: IntelligenceOperation,
        block_reason: IntelligenceBlockReason,
    ) -> Self {
        Self {
            contract_version: LOCAL_INTELLIGENCE_CONTRACT_VERSION,
            operation,
            tier: IntelligenceTier::Blocked,
            advisory_only: true,
            automatic_fallback_allowed: false,
            block_reason: Some(block_reason),
        }
    }
}

/// Routes a request without starting processes, allocating GPU memory, or publishing state.
pub fn route_request(request: &IntelligenceRequest) -> IntelligenceDecision {
    if request.contract_version != LOCAL_INTELLIGENCE_CONTRACT_VERSION {
        return IntelligenceDecision::blocked(
            request.operation,
            IntelligenceBlockReason::IncompatibleContractVersion,
        );
    }

    if request.input_bytes > MAXIMUM_REQUEST_BYTES {
        return IntelligenceDecision::blocked(
            request.operation,
            IntelligenceBlockReason::RequestTooLarge,
        );
    }

    let tier = match request.operation {
        IntelligenceOperation::MetadataInspection
        | IntelligenceOperation::GeometryValidation
        | IntelligenceOperation::EvidenceSummarization => IntelligenceTier::Deterministic,
        IntelligenceOperation::VisionAdjudication | IntelligenceOperation::FloorplanInterpretation => {
            IntelligenceTier::MistralRs
        }
        IntelligenceOperation::GeneralAssistance => {
            return IntelligenceDecision::blocked(
                request.operation,
                IntelligenceBlockReason::UnsupportedOperation,
            )
        }
    };

    IntelligenceDecision {
        contract_version: LOCAL_INTELLIGENCE_CONTRACT_VERSION,
        operation: request.operation,
        tier,
        advisory_only: request.requires_authoritative_state || tier != IntelligenceTier::Deterministic,
        automatic_fallback_allowed: false,
        block_reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(operation: IntelligenceOperation) -> IntelligenceRequest {
        IntelligenceRequest {
            contract_version: LOCAL_INTELLIGENCE_CONTRACT_VERSION,
            operation,
            input_bytes: 1024,
            requires_authoritative_state: false,
        }
    }

    #[test]
    fn deterministic_operations_do_not_allocate_an_inference_tier() {
        let decision = route_request(&request(IntelligenceOperation::GeometryValidation));
        assert_eq!(decision.tier, IntelligenceTier::Deterministic);
        assert!(!decision.advisory_only);
        assert!(!decision.automatic_fallback_allowed);
    }

    #[test]
    fn vision_is_routed_to_mistralrs_as_the_primary_runtime() {
        let decision = route_request(&request(IntelligenceOperation::VisionAdjudication));
        assert_eq!(decision.tier, IntelligenceTier::MistralRs);
        assert!(decision.advisory_only);
        assert!(!decision.automatic_fallback_allowed);
    }

    #[test]
    fn oversized_input_blocks_before_any_runtime_can_be_selected() {
        let mut request = request(IntelligenceOperation::FloorplanInterpretation);
        request.input_bytes = MAXIMUM_REQUEST_BYTES + 1;
        let decision = route_request(&request);
        assert_eq!(decision.tier, IntelligenceTier::Blocked);
        assert_eq!(
            decision.block_reason,
            Some(IntelligenceBlockReason::RequestTooLarge)
        );
    }

    #[test]
    fn incompatible_contract_version_fails_closed() {
        let mut request = request(IntelligenceOperation::MetadataInspection);
        request.contract_version += 1;
        let decision = route_request(&request);
        assert_eq!(decision.tier, IntelligenceTier::Blocked);
        assert_eq!(
            decision.block_reason,
            Some(IntelligenceBlockReason::IncompatibleContractVersion)
        );
    }

    #[test]
    fn authoritative_requests_are_advisory_even_when_deterministic() {
        let mut request = request(IntelligenceOperation::GeometryValidation);
        request.requires_authoritative_state = true;
        let decision = route_request(&request);
        assert_eq!(decision.tier, IntelligenceTier::Deterministic);
        assert!(decision.advisory_only);
    }

    #[test]
    fn llama_cpp_is_never_an_automatic_fallback() {
        let decision = route_request(&request(IntelligenceOperation::FloorplanInterpretation));
        assert_ne!(decision.tier, IntelligenceTier::LlamaCpp);
        assert!(!decision.automatic_fallback_allowed);
    }

    #[test]
    fn unsupported_assistance_is_blocked() {
        let decision = route_request(&request(IntelligenceOperation::GeneralAssistance));
        assert_eq!(decision.tier, IntelligenceTier::Blocked);
        assert_eq!(
            decision.block_reason,
            Some(IntelligenceBlockReason::UnsupportedOperation)
        );
    }
}
