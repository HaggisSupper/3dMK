use agentic_cad_backend::local_intelligence::{
    embedded_contract, evaluate_request, IntelligenceBlockReason, IntelligenceOperation,
    IntelligenceRequest, IntelligenceTier, LOCAL_INTELLIGENCE_CONTRACT_VERSION,
};

fn request(operation: IntelligenceOperation) -> IntelligenceRequest {
    IntelligenceRequest {
        contract_version: LOCAL_INTELLIGENCE_CONTRACT_VERSION,
        operation,
        input_bytes: 1024,
        requires_authoritative_state: false,
    }
}

#[test]
fn embedded_contract_is_typed_and_exposes_an_explicit_llamacpp_fallback_only() {
    let contract = embedded_contract().expect("embedded local-intelligence contract is valid");

    assert!(contract.invariants.deterministic_first);
    assert!(contract.invariants.mistralrs_primary);
    assert!(!contract.invariants.llamacpp_automatic_fallback);
    assert!(contract.invariants.llamacpp_explicit_fallback_available);
    assert!(!contract.invariants.cloud_egress_default_enabled);
}

#[test]
fn policy_evaluation_is_contract_driven_and_fails_closed_for_unsupported_work() {
    let decision = evaluate_request(&request(IntelligenceOperation::GeneralAssistance))
        .expect("embedded contract is valid");

    assert_eq!(decision.tier, IntelligenceTier::Blocked);
    assert_eq!(
        decision.block_reason,
        Some(IntelligenceBlockReason::UnsupportedOperation)
    );
}

#[test]
fn vision_selects_mistralrs_and_names_llamacpp_as_a_nonautomatic_contingency() {
    let decision = evaluate_request(&request(IntelligenceOperation::VisionAdjudication))
        .expect("embedded contract is valid");

    assert_eq!(decision.tier, IntelligenceTier::MistralRs);
    assert_eq!(
        decision.explicit_fallback_tier,
        Some(IntelligenceTier::LlamaCpp)
    );
    assert!(!decision.automatic_fallback_allowed);
}
