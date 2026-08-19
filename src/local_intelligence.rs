//! Contract-driven admission for the 3DMK local-intelligence ladder.
//!
//! This module decides policy only. It never starts a runtime, allocates GPU
//! memory, performs cloud egress, or publishes authoritative project state.
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use thiserror::Error;

pub const LOCAL_INTELLIGENCE_CONTRACT_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum IntelligenceOperation {
    MetadataInspection,
    GeometryValidation,
    EvidenceSummarization,
    VisionAdjudication,
    FloorplanInterpretation,
    GeneralAssistance,
}

impl IntelligenceOperation {
    const ALL: [Self; 6] = [
        Self::MetadataInspection,
        Self::GeometryValidation,
        Self::EvidenceSummarization,
        Self::VisionAdjudication,
        Self::FloorplanInterpretation,
        Self::GeneralAssistance,
    ];
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
    pub explicit_fallback_tier: Option<IntelligenceTier>,
    pub advisory_only: bool,
    pub automatic_fallback_allowed: bool,
    pub block_reason: Option<IntelligenceBlockReason>,
}

impl IntelligenceDecision {
    const fn blocked(
        contract_version: u16,
        operation: IntelligenceOperation,
        block_reason: IntelligenceBlockReason,
    ) -> Self {
        Self {
            contract_version,
            operation,
            tier: IntelligenceTier::Blocked,
            explicit_fallback_tier: None,
            advisory_only: true,
            automatic_fallback_allowed: false,
            block_reason: Some(block_reason),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LocalIntelligenceRoutingContract {
    pub contract_version: u16,
    pub component: String,
    pub operations: Vec<ContractOperationRoute>,
    pub limits: ContractLimits,
    pub invariants: ContractInvariants,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ContractOperationRoute {
    pub operation: IntelligenceOperation,
    pub primary_tier: IntelligenceTier,
    pub explicit_fallback_tier: Option<IntelligenceTier>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ContractLimits {
    pub maximum_request_bytes: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ContractInvariants {
    pub deterministic_first: bool,
    pub mistralrs_primary: bool,
    pub llamacpp_automatic_fallback: bool,
    pub llamacpp_explicit_fallback_available: bool,
    pub cloud_egress_default_enabled: bool,
    pub local_cpu_inference_allowed: bool,
    pub ai_can_publish_authoritative_state: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IntelligenceContractError {
    #[error("local-intelligence contract is invalid JSON: {0}")]
    InvalidJson(String),
    #[error("unsupported local-intelligence contract version {0}")]
    UnsupportedVersion(u16),
    #[error("local-intelligence contract component must not be blank")]
    BlankComponent,
    #[error("local-intelligence contract request budget must be non-zero")]
    ZeroRequestBudget,
    #[error("local-intelligence contract is missing route for {0:?}")]
    MissingOperation(IntelligenceOperation),
    #[error("local-intelligence contract duplicates route for {0:?}")]
    DuplicateOperation(IntelligenceOperation),
    #[error("local-intelligence contract has invalid route for {0:?}")]
    InvalidRoute(IntelligenceOperation),
    #[error("local-intelligence contract violates invariant {0}")]
    InvalidInvariant(&'static str),
}

impl LocalIntelligenceRoutingContract {
    fn validate(self) -> Result<Self, IntelligenceContractError> {
        if self.contract_version != LOCAL_INTELLIGENCE_CONTRACT_VERSION {
            return Err(IntelligenceContractError::UnsupportedVersion(
                self.contract_version,
            ));
        }
        if self.component.trim().is_empty() {
            return Err(IntelligenceContractError::BlankComponent);
        }
        if self.limits.maximum_request_bytes == 0 {
            return Err(IntelligenceContractError::ZeroRequestBudget);
        }

        let invariants = self.invariants;
        if !invariants.deterministic_first {
            return Err(IntelligenceContractError::InvalidInvariant(
                "deterministic_first",
            ));
        }
        if !invariants.mistralrs_primary {
            return Err(IntelligenceContractError::InvalidInvariant("mistralrs_primary"));
        }
        if invariants.llamacpp_automatic_fallback {
            return Err(IntelligenceContractError::InvalidInvariant(
                "llamacpp_automatic_fallback",
            ));
        }
        if !invariants.llamacpp_explicit_fallback_available {
            return Err(IntelligenceContractError::InvalidInvariant(
                "llamacpp_explicit_fallback_available",
            ));
        }
        if invariants.cloud_egress_default_enabled {
            return Err(IntelligenceContractError::InvalidInvariant(
                "cloud_egress_default_enabled",
            ));
        }
        if invariants.local_cpu_inference_allowed {
            return Err(IntelligenceContractError::InvalidInvariant(
                "local_cpu_inference_allowed",
            ));
        }
        if invariants.ai_can_publish_authoritative_state {
            return Err(IntelligenceContractError::InvalidInvariant(
                "ai_can_publish_authoritative_state",
            ));
        }

        let mut seen = BTreeSet::new();
        for route in &self.operations {
            if !seen.insert(route.operation) {
                return Err(IntelligenceContractError::DuplicateOperation(route.operation));
            }
            let valid = match route.operation {
                IntelligenceOperation::MetadataInspection
                | IntelligenceOperation::GeometryValidation
                | IntelligenceOperation::EvidenceSummarization => {
                    route.primary_tier == IntelligenceTier::Deterministic
                        && route.explicit_fallback_tier.is_none()
                }
                IntelligenceOperation::VisionAdjudication
                | IntelligenceOperation::FloorplanInterpretation => {
                    route.primary_tier == IntelligenceTier::MistralRs
                        && route.explicit_fallback_tier == Some(IntelligenceTier::LlamaCpp)
                }
                IntelligenceOperation::GeneralAssistance => {
                    route.primary_tier == IntelligenceTier::Blocked
                        && route.explicit_fallback_tier.is_none()
                }
            };
            if !valid {
                return Err(IntelligenceContractError::InvalidRoute(route.operation));
            }
        }

        for operation in IntelligenceOperation::ALL {
            if !seen.contains(&operation) {
                return Err(IntelligenceContractError::MissingOperation(operation));
            }
        }
        Ok(self)
    }

    fn route(&self, operation: IntelligenceOperation) -> &ContractOperationRoute {
        self.operations
            .iter()
            .find(|route| route.operation == operation)
            .expect("validated local-intelligence contract contains every operation")
    }
}

pub fn parse_contract(
    json: &str,
) -> Result<LocalIntelligenceRoutingContract, IntelligenceContractError> {
    serde_json::from_str(json)
        .map_err(|error| IntelligenceContractError::InvalidJson(error.to_string()))?
        .validate()
}

pub fn embedded_contract() -> Result<LocalIntelligenceRoutingContract, IntelligenceContractError> {
    parse_contract(include_str!("../contracts/local-intelligence-routing.v1.json"))
}

/// Evaluates a request with the embedded immutable routing contract.
///
/// No runtime is started and no authoritative state can be altered by this call.
pub fn evaluate_request(
    request: &IntelligenceRequest,
) -> Result<IntelligenceDecision, IntelligenceContractError> {
    let contract = embedded_contract()?;
    if request.contract_version != contract.contract_version {
        return Ok(IntelligenceDecision::blocked(
            contract.contract_version,
            request.operation,
            IntelligenceBlockReason::IncompatibleContractVersion,
        ));
    }
    if request.input_bytes > contract.limits.maximum_request_bytes {
        return Ok(IntelligenceDecision::blocked(
            contract.contract_version,
            request.operation,
            IntelligenceBlockReason::RequestTooLarge,
        ));
    }

    let route = contract.route(request.operation);
    if route.primary_tier == IntelligenceTier::Blocked {
        return Ok(IntelligenceDecision::blocked(
            contract.contract_version,
            request.operation,
            IntelligenceBlockReason::UnsupportedOperation,
        ));
    }

    Ok(IntelligenceDecision {
        contract_version: contract.contract_version,
        operation: request.operation,
        tier: route.primary_tier,
        explicit_fallback_tier: route.explicit_fallback_tier,
        advisory_only: request.requires_authoritative_state
            || route.primary_tier != IntelligenceTier::Deterministic,
        automatic_fallback_allowed: false,
        block_reason: None,
    })
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IntelligenceCapacity {
    pub cuda_ready: bool,
    pub available_vram_mib: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IntelligenceGovernorConfig {
    pub maximum_active_inference: usize,
    pub minimum_inference_vram_mib: u32,
    pub interactive_safety_reserve_mib: u32,
}

impl Default for IntelligenceGovernorConfig {
    fn default() -> Self {
        Self {
            maximum_active_inference: 1,
            minimum_inference_vram_mib: 1536,
            interactive_safety_reserve_mib: 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntelligenceAdmissionBlockReason {
    RoutingBlocked,
    CudaUnavailable,
    InsufficientVram,
    ConcurrencyLimitReached,
}

#[derive(Debug, Error)]
pub enum IntelligenceAdmissionError {
    #[error(transparent)]
    Contract(#[from] IntelligenceContractError),
    #[error("local-intelligence admission blocked: {0:?}")]
    Blocked(IntelligenceAdmissionBlockReason),
}

#[derive(Debug)]
pub struct IntelligenceGovernor {
    config: IntelligenceGovernorConfig,
    active_inference: Arc<AtomicUsize>,
}

impl IntelligenceGovernor {
    pub fn new(config: IntelligenceGovernorConfig) -> Self {
        Self {
            config,
            active_inference: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn active_inference(&self) -> usize {
        self.active_inference.load(Ordering::Acquire)
    }

    pub fn admit(
        &self,
        request: &IntelligenceRequest,
        capacity: IntelligenceCapacity,
    ) -> Result<IntelligenceLease, IntelligenceAdmissionError> {
        let decision = evaluate_request(request)?;
        match decision.tier {
            IntelligenceTier::Blocked => {
                return Err(IntelligenceAdmissionError::Blocked(
                    IntelligenceAdmissionBlockReason::RoutingBlocked,
                ));
            }
            IntelligenceTier::Deterministic => return Ok(IntelligenceLease::deterministic()),
            IntelligenceTier::MistralRs
            | IntelligenceTier::LlamaCpp
            | IntelligenceTier::OpenAiCompatibleCloud => {}
        }

        if !capacity.cuda_ready {
            return Err(IntelligenceAdmissionError::Blocked(
                IntelligenceAdmissionBlockReason::CudaUnavailable,
            ));
        }

        let required_vram = self
            .config
            .minimum_inference_vram_mib
            .saturating_add(self.config.interactive_safety_reserve_mib);
        if capacity.available_vram_mib < required_vram {
            return Err(IntelligenceAdmissionError::Blocked(
                IntelligenceAdmissionBlockReason::InsufficientVram,
            ));
        }

        let mut active = self.active_inference.load(Ordering::Acquire);
        loop {
            if active >= self.config.maximum_active_inference {
                return Err(IntelligenceAdmissionError::Blocked(
                    IntelligenceAdmissionBlockReason::ConcurrencyLimitReached,
                ));
            }
            match self.active_inference.compare_exchange_weak(
                active,
                active + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Ok(IntelligenceLease {
                        active_inference: Some(Arc::clone(&self.active_inference)),
                    });
                }
                Err(observed) => active = observed,
            }
        }
    }
}

#[derive(Debug)]
pub struct IntelligenceLease {
    active_inference: Option<Arc<AtomicUsize>>,
}

impl IntelligenceLease {
    const fn deterministic() -> Self {
        Self {
            active_inference: None,
        }
    }
}

impl Drop for IntelligenceLease {
    fn drop(&mut self) {
        if let Some(active_inference) = self.active_inference.take() {
            active_inference.fetch_sub(1, Ordering::AcqRel);
        }
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
    fn contract_rejects_silent_fallback_drift() {
        let invalid = include_str!("../contracts/local-intelligence-routing.v1.json")
            .replace("\"llamacpp_automatic_fallback\": false", "\"llamacpp_automatic_fallback\": true");
        assert!(matches!(
            parse_contract(&invalid),
            Err(IntelligenceContractError::InvalidInvariant(
                "llamacpp_automatic_fallback"
            ))
        ));
    }

    #[test]
    fn contract_rejects_missing_operation_drift() {
        let invalid = r#"{
            "contract_version": 1,
            "component": "test",
            "operations": [],
            "limits": { "maximum_request_bytes": 1 },
            "invariants": {
                "deterministic_first": true,
                "mistralrs_primary": true,
                "llamacpp_automatic_fallback": false,
                "llamacpp_explicit_fallback_available": true,
                "cloud_egress_default_enabled": false,
                "local_cpu_inference_allowed": false,
                "ai_can_publish_authoritative_state": false
            }
        }"#;
        assert!(matches!(
            parse_contract(invalid),
            Err(IntelligenceContractError::MissingOperation(_))
        ));
    }

    #[test]
    fn governor_reserves_vram_and_releases_the_concurrency_lease() {
        let governor = IntelligenceGovernor::new(IntelligenceGovernorConfig {
            maximum_active_inference: 1,
            minimum_inference_vram_mib: 2048,
            interactive_safety_reserve_mib: 1024,
        });
        let request = request(IntelligenceOperation::VisionAdjudication);
        let capacity = IntelligenceCapacity {
            cuda_ready: true,
            available_vram_mib: 4096,
        };

        let lease = governor.admit(&request, capacity).unwrap();
        assert_eq!(governor.active_inference(), 1);
        assert!(matches!(
            governor.admit(&request, capacity),
            Err(IntelligenceAdmissionError::Blocked(
                IntelligenceAdmissionBlockReason::ConcurrencyLimitReached
            ))
        ));
        drop(lease);
        assert_eq!(governor.active_inference(), 0);
        assert!(governor.admit(&request, capacity).is_ok());
    }

    #[test]
    fn governor_blocks_local_inference_without_cuda_or_safe_vram() {
        let governor = IntelligenceGovernor::new(IntelligenceGovernorConfig::default());
        let request = request(IntelligenceOperation::FloorplanInterpretation);
        assert!(matches!(
            governor.admit(
                &request,
                IntelligenceCapacity {
                    cuda_ready: false,
                    available_vram_mib: 8192,
                },
            ),
            Err(IntelligenceAdmissionError::Blocked(
                IntelligenceAdmissionBlockReason::CudaUnavailable
            ))
        ));
        assert!(matches!(
            governor.admit(
                &request,
                IntelligenceCapacity {
                    cuda_ready: true,
                    available_vram_mib: 2048,
                },
            ),
            Err(IntelligenceAdmissionError::Blocked(
                IntelligenceAdmissionBlockReason::InsufficientVram
            ))
        ));
    }

    #[test]
    fn deterministic_work_never_consumes_gpu_capacity() {
        let governor = IntelligenceGovernor::new(IntelligenceGovernorConfig::default());
        let lease = governor
            .admit(
                &request(IntelligenceOperation::EvidenceSummarization),
                IntelligenceCapacity {
                    cuda_ready: false,
                    available_vram_mib: 0,
                },
            )
            .unwrap();
        assert_eq!(governor.active_inference(), 0);
        drop(lease);
    }

    #[test]
    fn requests_fail_closed_before_runtime_selection() {
        let mut too_large = request(IntelligenceOperation::FloorplanInterpretation);
        too_large.input_bytes = embedded_contract().unwrap().limits.maximum_request_bytes + 1;
        assert_eq!(
            evaluate_request(&too_large).unwrap().block_reason,
            Some(IntelligenceBlockReason::RequestTooLarge)
        );

        let mut incompatible = request(IntelligenceOperation::MetadataInspection);
        incompatible.contract_version += 1;
        assert_eq!(
            evaluate_request(&incompatible).unwrap().block_reason,
            Some(IntelligenceBlockReason::IncompatibleContractVersion)
        );
    }

    #[test]
    fn mistralrs_is_primary_and_llamacpp_is_explicit_only() {
        let decision = evaluate_request(&request(IntelligenceOperation::VisionAdjudication)).unwrap();
        assert_eq!(decision.tier, IntelligenceTier::MistralRs);
        assert_eq!(decision.explicit_fallback_tier, Some(IntelligenceTier::LlamaCpp));
        assert!(!decision.automatic_fallback_allowed);
        assert!(decision.advisory_only);
    }
}
