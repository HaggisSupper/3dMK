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

#[derive(Debug)]
pub struct IntelligenceGovernor {
    config: IntelligenceGovernorConfig,
    active_inference: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl IntelligenceGovernor {
    pub fn new(config: IntelligenceGovernorConfig) -> Self {
        Self {
            config,
            active_inference: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    pub fn active_inference(&self) -> usize {
        self.active_inference.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn admit(
        &self,
        request: &IntelligenceRequest,
        capacity: IntelligenceCapacity,
    ) -> Result<IntelligenceLease, IntelligenceAdmissionBlockReason> {
        let decision = route_request(request);
        match decision.tier {
            IntelligenceTier::Blocked => {
                return Err(IntelligenceAdmissionBlockReason::RoutingBlocked)
            }
            IntelligenceTier::Deterministic => {
                return Ok(IntelligenceLease::deterministic());
            }
            IntelligenceTier::MistralRs
            | IntelligenceTier::LlamaCpp
            | IntelligenceTier::OpenAiCompatibleCloud => {}
        }

        if !capacity.cuda_ready {
            return Err(IntelligenceAdmissionBlockReason::CudaUnavailable);
        }

        let required_vram = self
            .config
            .minimum_inference_vram_mib
            .saturating_add(self.config.interactive_safety_reserve_mib);
        if capacity.available_vram_mib < required_vram {
            return Err(IntelligenceAdmissionBlockReason::InsufficientVram);
        }

        let mut active = self.active_inference.load(std::sync::atomic::Ordering::Acquire);
        loop {
            if active >= self.config.maximum_active_inference {
                return Err(IntelligenceAdmissionBlockReason::ConcurrencyLimitReached);
            }
            match self.active_inference.compare_exchange_weak(
                active,
                active + 1,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Ok(IntelligenceLease {
                        active_inference: Some(std::sync::Arc::clone(&self.active_inference)),
                    })
                }
                Err(observed) => active = observed,
            }
        }
    }
}

#[derive(Debug)]
pub struct IntelligenceLease {
    active_inference: Option<std::sync::Arc<std::sync::atomic::AtomicUsize>>,
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
            active_inference.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
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
            Err(IntelligenceAdmissionBlockReason::ConcurrencyLimitReached)
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
            Err(IntelligenceAdmissionBlockReason::CudaUnavailable)
        ));
        assert!(matches!(
            governor.admit(
                &request,
                IntelligenceCapacity {
                    cuda_ready: true,
                    available_vram_mib: 2048,
                },
            ),
            Err(IntelligenceAdmissionBlockReason::InsufficientVram)
        ));
    }

    #[test]
    fn governor_never_consumes_gpu_capacity_for_deterministic_work() {
        let governor = IntelligenceGovernor::new(IntelligenceGovernorConfig::default());
        let request = request(IntelligenceOperation::EvidenceSummarization);
        let lease = governor
            .admit(
                &request,
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
