use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DesktopTargetId(String);

impl DesktopTargetId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DesktopTargetId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesktopPlatform {
    Windows,
    Macos,
    Linux,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopTargetPolicy {
    pub id: DesktopTargetId,
    pub platform: DesktopPlatform,
    pub process_name: String,
    pub executable_hash_sha256: Option<String>,
    pub window_class: Option<String>,
    pub capability_profile: DesktopCapabilityProfile,
    pub authorization: DesktopAuthorization,
    pub minimum_tree_completeness: AccessibilityTreeCompleteness,
    pub allowed_observation_modes: Vec<DesktopObservationMode>,
    pub allowed_action_classes: Vec<DesktopActionClass>,
    pub coordinate_fallback: CoordinateFallbackPolicy,
}

impl DesktopTargetPolicy {
    pub fn allows_observation_mode(&self, mode: DesktopObservationMode) -> bool {
        self.allowed_observation_modes.contains(&mode)
    }

    pub fn allows_action_class(&self, class: DesktopActionClass) -> bool {
        self.allowed_action_classes.contains(&class)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopCapabilityProfile {
    pub profile_id: String,
    pub assessed_at: OffsetDateTime,
    pub tree_completeness: AccessibilityTreeCompleteness,
    pub semantic_actions: SemanticActionCapability,
    pub capture_time_phi_handling: CaptureTimePhiHandling,
    pub visual_fallback: VisualFallbackCapability,
}

impl DesktopCapabilityProfile {
    pub fn allows_action_class(&self, action_class: DesktopActionClass) -> bool {
        match self.semantic_actions {
            SemanticActionCapability::Unknown
            | SemanticActionCapability::NotSupported
            | SemanticActionCapability::ObserveOnly => false,
            SemanticActionCapability::NonIrreversibleActions => !action_class.is_irreversible(),
            SemanticActionCapability::IrreversibleActionsWithConfirmation => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticActionCapability {
    Unknown,
    NotSupported,
    ObserveOnly,
    NonIrreversibleActions,
    IrreversibleActionsWithConfirmation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureTimePhiHandling {
    Unknown,
    Required,
    Verified,
}

impl CaptureTimePhiHandling {
    pub fn can_capture_safely(self) -> bool {
        matches!(self, Self::Required | Self::Verified)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualFallbackCapability {
    Disabled,
    ObserveOnly,
    ExplicitPolicyRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopAuthorization {
    pub authorized_by: String,
    pub authorized_at: OffsetDateTime,
    pub expires_at: Option<OffsetDateTime>,
    pub reason: String,
}

impl DesktopAuthorization {
    pub fn is_active_at(&self, now: OffsetDateTime) -> bool {
        match self.expires_at {
            Some(expires_at) => expires_at > now,
            None => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AccessibilityTreeCompleteness {
    Unknown,
    Empty,
    Partial,
    SemanticControlsPresent,
    VerifiedComplete,
}

impl AccessibilityTreeCompleteness {
    pub fn satisfies(self, required: Self) -> bool {
        self >= required
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesktopObservationMode {
    AccessibilityTree,
    ScreenshotMetadata,
    OcrText,
    Hybrid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoordinateFallbackPolicy {
    Never,
    ObserveOnly,
    AllowNonIrreversibleWithApproval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhiCapturePolicy {
    SuppressRawCapture,
    RedactBeforePersist,
    SessionOnlyNoPersist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopAutomationPolicy {
    supported_platforms: Vec<DesktopPlatform>,
    targets: HashMap<DesktopTargetId, DesktopTargetPolicy>,
}

impl DesktopAutomationPolicy {
    pub fn empty() -> Self {
        Self {
            supported_platforms: vec![DesktopPlatform::Macos],
            targets: HashMap::new(),
        }
    }

    pub fn with_supported_platforms(platforms: impl IntoIterator<Item = DesktopPlatform>) -> Self {
        Self {
            supported_platforms: platforms.into_iter().collect(),
            targets: HashMap::new(),
        }
    }

    pub fn with_targets(targets: impl IntoIterator<Item = DesktopTargetPolicy>) -> Self {
        let mut policy = Self::empty();
        for target in targets {
            policy.register_target(target);
        }
        policy
    }

    pub fn register_target(&mut self, target: DesktopTargetPolicy) {
        self.targets.insert(target.id.clone(), target);
    }

    pub fn target(&self, target_id: &DesktopTargetId) -> Option<&DesktopTargetPolicy> {
        self.targets.get(target_id)
    }

    pub fn supports_platform(&self, platform: DesktopPlatform) -> bool {
        self.supported_platforms.contains(&platform)
    }

    pub fn evaluate_observation_request(
        &self,
        request: &DesktopObservationRequest,
        now: OffsetDateTime,
    ) -> Result<(), DesktopAutomationError> {
        let target = self.require_target(&request.target_id)?;
        self.evaluate_target_scope(target)?;

        if !target.authorization.is_active_at(now) {
            return Err(DesktopAutomationError::AuthorizationExpired(
                request.target_id.clone(),
            ));
        }

        if !target.allows_observation_mode(request.mode) {
            return Err(DesktopAutomationError::ObservationModeBlocked {
                target_id: request.target_id.clone(),
                mode: request.mode,
            });
        }

        Ok(())
    }

    pub fn evaluate_observation(
        &self,
        observation: &DesktopObservation,
    ) -> Result<(), DesktopAutomationError> {
        let target = self.require_target(&observation.target_id)?;
        self.evaluate_target_scope(target)?;

        if !observation
            .tree_completeness
            .satisfies(target.minimum_tree_completeness)
        {
            return Err(DesktopAutomationError::TreeCompletenessTooLow {
                target_id: observation.target_id.clone(),
                observed: observation.tree_completeness,
                required: target.minimum_tree_completeness,
            });
        }

        if !observation.capture.is_phi_safe() {
            return Err(DesktopAutomationError::PhiCaptureNotHandledAtCapture(
                observation.observation_id.clone(),
            ));
        }

        if !target
            .capability_profile
            .capture_time_phi_handling
            .can_capture_safely()
        {
            return Err(DesktopAutomationError::CapabilityProfileLacksPhiCapture(
                target.id.clone(),
            ));
        }

        Ok(())
    }

    pub fn evaluate_proposal(
        &self,
        proposal: &DesktopActionProposal,
        now: OffsetDateTime,
    ) -> Result<(), DesktopAutomationError> {
        let target = self.require_target(&proposal.target_id)?;
        self.evaluate_target_scope(target)?;

        if !target.authorization.is_active_at(now) {
            return Err(DesktopAutomationError::AuthorizationExpired(
                proposal.target_id.clone(),
            ));
        }

        let action_class = proposal.action.class();
        if !target.allows_action_class(action_class) {
            return Err(DesktopAutomationError::ActionClassBlocked {
                target_id: proposal.target_id.clone(),
                action_class,
            });
        }

        if !target.capability_profile.allows_action_class(action_class) {
            return Err(DesktopAutomationError::CapabilityProfileBlocksAction {
                target_id: proposal.target_id.clone(),
                action_class,
            });
        }

        proposal.bounds.validate()?;

        if proposal.action.uses_coordinate_selector() {
            match target.coordinate_fallback {
                CoordinateFallbackPolicy::Never | CoordinateFallbackPolicy::ObserveOnly => {
                    return Err(DesktopAutomationError::CoordinateFallbackBlocked(
                        proposal.proposal_id.clone(),
                    ));
                }
                CoordinateFallbackPolicy::AllowNonIrreversibleWithApproval => {
                    if proposal.action.irreversible_kind().is_some() {
                        return Err(DesktopAutomationError::CoordinateFallbackBlocked(
                            proposal.proposal_id.clone(),
                        ));
                    }
                }
            }
        }

        if proposal.action.irreversible_kind().is_some() && !proposal.requires_human_confirmation {
            return Err(DesktopAutomationError::HumanConfirmationRequired(
                proposal.proposal_id.clone(),
            ));
        }

        Ok(())
    }

    pub fn evaluate_approval(
        &self,
        proposal: &DesktopActionProposal,
        approval: &DesktopActionApproval,
    ) -> Result<(), DesktopAutomationError> {
        if proposal.proposal_id != approval.proposal_id {
            return Err(DesktopAutomationError::ApprovalProposalMismatch {
                proposal_id: proposal.proposal_id.clone(),
                approval_proposal_id: approval.proposal_id.clone(),
            });
        }

        if approval.decision != DesktopApprovalDecision::Approved {
            return Err(DesktopAutomationError::ProposalNotApproved(
                proposal.proposal_id.clone(),
            ));
        }

        if let Some(irreversible_kind) = proposal.action.irreversible_kind() {
            if approval.irreversible_confirmation != Some(irreversible_kind) {
                return Err(DesktopAutomationError::IrreversibleConfirmationRequired {
                    proposal_id: proposal.proposal_id.clone(),
                    irreversible_kind,
                });
            }
        }

        Ok(())
    }

    fn require_target(
        &self,
        target_id: &DesktopTargetId,
    ) -> Result<&DesktopTargetPolicy, DesktopAutomationError> {
        self.targets
            .get(target_id)
            .ok_or_else(|| DesktopAutomationError::TargetNotAllowlisted(target_id.clone()))
    }

    fn evaluate_target_scope(
        &self,
        target: &DesktopTargetPolicy,
    ) -> Result<(), DesktopAutomationError> {
        if !self.supports_platform(target.platform) {
            return Err(DesktopAutomationError::PlatformOutOfScope {
                target_id: target.id.clone(),
                platform: target.platform,
            });
        }

        if !target
            .capability_profile
            .tree_completeness
            .satisfies(target.minimum_tree_completeness)
        {
            return Err(DesktopAutomationError::CapabilityProfileTooWeak {
                target_id: target.id.clone(),
                observed: target.capability_profile.tree_completeness,
                required: target.minimum_tree_completeness,
            });
        }

        Ok(())
    }
}

impl Default for DesktopAutomationPolicy {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopObservationRequest {
    pub run_id: String,
    pub target_id: DesktopTargetId,
    pub mode: DesktopObservationMode,
    pub capture_policy: PhiCapturePolicy,
    pub requested_at: OffsetDateTime,
    pub redacted_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopObservation {
    pub observation_id: String,
    pub run_id: String,
    pub target_id: DesktopTargetId,
    pub observed_at: OffsetDateTime,
    pub mode: DesktopObservationMode,
    pub tree_completeness: AccessibilityTreeCompleteness,
    pub capture: DesktopCaptureSummary,
    pub controls: Vec<ObservedControl>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopCaptureSummary {
    pub raw_capture_persisted: bool,
    pub phi_handled_at_capture: bool,
    pub visual_fallback_used: bool,
}

impl DesktopCaptureSummary {
    pub fn phi_safe_without_raw_persistence() -> Self {
        Self {
            raw_capture_persisted: false,
            phi_handled_at_capture: true,
            visual_fallback_used: false,
        }
    }

    pub fn is_phi_safe(self) -> bool {
        !self.raw_capture_persisted && self.phi_handled_at_capture
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedControl {
    pub control_id: String,
    pub role: ControlRole,
    pub label_hash_sha256: Option<String>,
    pub bounds: Option<ScreenRect>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlRole {
    Button,
    Checkbox,
    ComboBox,
    MenuItem,
    TextInput,
    Table,
    Window,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopActionProposal {
    pub proposal_id: String,
    pub run_id: String,
    pub target_id: DesktopTargetId,
    pub observation_id: String,
    pub action: DesktopActionKind,
    pub risk: DesktopActionRisk,
    pub created_at: OffsetDateTime,
    pub redacted_rationale: String,
    pub bounds: AutomationBounds,
    pub requires_human_confirmation: bool,
    pub verification: DesktopVerificationExpectation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DesktopActionKind {
    Click {
        selector: ControlSelector,
    },
    EnterText {
        selector: ControlSelector,
        value: SensitiveValueRef,
    },
    KeyChord {
        chord: Vec<String>,
    },
    SelectOption {
        selector: ControlSelector,
        option: SensitiveValueRef,
    },
    Wait {
        milliseconds: u64,
    },
    IrreversibleCommand {
        selector: ControlSelector,
        kind: IrreversibleActionKind,
    },
}

impl DesktopActionKind {
    pub fn class(&self) -> DesktopActionClass {
        match self {
            Self::Click { .. } => DesktopActionClass::Click,
            Self::EnterText { .. } => DesktopActionClass::EnterText,
            Self::KeyChord { .. } => DesktopActionClass::KeyChord,
            Self::SelectOption { .. } => DesktopActionClass::SelectOption,
            Self::Wait { .. } => DesktopActionClass::Wait,
            Self::IrreversibleCommand { kind, .. } => (*kind).into(),
        }
    }

    pub fn irreversible_kind(&self) -> Option<IrreversibleActionKind> {
        match self {
            Self::IrreversibleCommand { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    pub fn uses_coordinate_selector(&self) -> bool {
        match self {
            Self::Click { selector }
            | Self::EnterText { selector, .. }
            | Self::SelectOption { selector, .. }
            | Self::IrreversibleCommand { selector, .. } => selector.is_coordinate(),
            Self::KeyChord { .. } | Self::Wait { .. } => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesktopActionClass {
    Click,
    EnterText,
    KeyChord,
    SelectOption,
    Wait,
    Sign,
    Submit,
    Export,
    Delete,
    Finalize,
    OtherIrreversible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrreversibleActionKind {
    Sign,
    Submit,
    Export,
    Delete,
    Finalize,
    Other,
}

impl From<IrreversibleActionKind> for DesktopActionClass {
    fn from(value: IrreversibleActionKind) -> Self {
        match value {
            IrreversibleActionKind::Sign => Self::Sign,
            IrreversibleActionKind::Submit => Self::Submit,
            IrreversibleActionKind::Export => Self::Export,
            IrreversibleActionKind::Delete => Self::Delete,
            IrreversibleActionKind::Finalize => Self::Finalize,
            IrreversibleActionKind::Other => Self::OtherIrreversible,
        }
    }
}

impl DesktopActionClass {
    pub fn is_irreversible(self) -> bool {
        matches!(
            self,
            Self::Sign
                | Self::Submit
                | Self::Export
                | Self::Delete
                | Self::Finalize
                | Self::OtherIrreversible
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesktopActionRisk {
    Low,
    Moderate,
    High,
    Irreversible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlSelector {
    Accessibility {
        control_id: String,
        role: ControlRole,
        label_hash_sha256: Option<String>,
    },
    Coordinates {
        rect: ScreenRect,
        redacted_reason: String,
    },
}

impl ControlSelector {
    pub fn is_coordinate(&self) -> bool {
        matches!(self, Self::Coordinates { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensitiveValueRef {
    pub local_ref: String,
    pub redacted_preview: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationBounds {
    pub max_steps: u32,
    pub max_wall_clock_seconds: u64,
}

impl AutomationBounds {
    pub fn validate(self) -> Result<(), DesktopAutomationError> {
        if self.max_steps == 0 || self.max_wall_clock_seconds == 0 {
            return Err(DesktopAutomationError::UnboundedRun);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopActionApproval {
    pub approval_id: String,
    pub proposal_id: String,
    pub approved_by: String,
    pub approved_at: OffsetDateTime,
    pub decision: DesktopApprovalDecision,
    pub irreversible_confirmation: Option<IrreversibleActionKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesktopApprovalDecision {
    Approved,
    Denied,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopActionResult {
    pub run_id: String,
    pub proposal_id: String,
    pub attempted_at: OffsetDateTime,
    pub completed_at: Option<OffsetDateTime>,
    pub status: DesktopActionStatus,
    pub redacted_summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesktopActionStatus {
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopVerificationRequest {
    pub run_id: String,
    pub target_id: DesktopTargetId,
    pub proposal_id: String,
    pub expectation: DesktopVerificationExpectation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesktopVerificationExpectation {
    ControlExists {
        selector: ControlSelector,
    },
    ControlStateChanged {
        selector: ControlSelector,
        redacted_expected_state: String,
    },
    WorkflowStateChanged {
        redacted_expected_state: String,
    },
    NoExpectation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopVerification {
    pub request: DesktopVerificationRequest,
    pub verified_at: OffsetDateTime,
    pub status: DesktopVerificationStatus,
    pub redacted_summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesktopVerificationStatus {
    Passed,
    Failed,
    Inconclusive,
}

pub trait DesktopObserver {
    fn observe(
        &self,
        request: &DesktopObservationRequest,
    ) -> Result<DesktopObservation, DesktopAutomationError>;
}

pub trait DesktopActionProposer {
    fn propose_action(
        &self,
        observation: &DesktopObservation,
        intent: &DesktopActionIntent,
    ) -> Result<DesktopActionProposal, DesktopAutomationError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopActionIntent {
    pub run_id: String,
    pub target_id: DesktopTargetId,
    pub redacted_goal: String,
    pub allowed_action_classes: Vec<DesktopActionClass>,
    pub bounds: AutomationBounds,
}

pub trait DesktopActor {
    fn act(
        &self,
        proposal: &DesktopActionProposal,
        approval: &DesktopActionApproval,
    ) -> Result<DesktopActionResult, DesktopAutomationError>;
}

pub trait DesktopVerifier {
    fn verify(
        &self,
        request: &DesktopVerificationRequest,
    ) -> Result<DesktopVerification, DesktopAutomationError>;
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DesktopAutomationError {
    #[error("desktop automation target is not allowlisted: {0}")]
    TargetNotAllowlisted(DesktopTargetId),

    #[error("desktop automation platform {platform:?} is out of scope for target: {target_id}")]
    PlatformOutOfScope {
        target_id: DesktopTargetId,
        platform: DesktopPlatform,
    },

    #[error("desktop automation authorization expired for target: {0}")]
    AuthorizationExpired(DesktopTargetId),

    #[error("desktop observation mode {mode:?} is blocked for target: {target_id}")]
    ObservationModeBlocked {
        target_id: DesktopTargetId,
        mode: DesktopObservationMode,
    },

    #[error(
        "desktop accessibility tree completeness for target {target_id} is {observed:?}, below required {required:?}"
    )]
    TreeCompletenessTooLow {
        target_id: DesktopTargetId,
        observed: AccessibilityTreeCompleteness,
        required: AccessibilityTreeCompleteness,
    },

    #[error(
        "desktop capability profile for target {target_id} has tree completeness {observed:?}, below required {required:?}"
    )]
    CapabilityProfileTooWeak {
        target_id: DesktopTargetId,
        observed: AccessibilityTreeCompleteness,
        required: AccessibilityTreeCompleteness,
    },

    #[error("desktop capability profile lacks capture-time PHI handling for target: {0}")]
    CapabilityProfileLacksPhiCapture(DesktopTargetId),

    #[error("desktop observation did not handle PHI at capture time: {0}")]
    PhiCaptureNotHandledAtCapture(String),

    #[error("desktop action class {action_class:?} is blocked for target: {target_id}")]
    ActionClassBlocked {
        target_id: DesktopTargetId,
        action_class: DesktopActionClass,
    },

    #[error(
        "desktop capability profile blocks action class {action_class:?} for target: {target_id}"
    )]
    CapabilityProfileBlocksAction {
        target_id: DesktopTargetId,
        action_class: DesktopActionClass,
    },

    #[error("desktop automation run must have nonzero step and time bounds")]
    UnboundedRun,

    #[error("desktop coordinate fallback is blocked for proposal: {0}")]
    CoordinateFallbackBlocked(String),

    #[error("desktop proposal requires human confirmation: {0}")]
    HumanConfirmationRequired(String),

    #[error(
        "desktop approval references proposal {approval_proposal_id}, not expected proposal {proposal_id}"
    )]
    ApprovalProposalMismatch {
        proposal_id: String,
        approval_proposal_id: String,
    },

    #[error("desktop proposal was not approved: {0}")]
    ProposalNotApproved(String),

    #[error(
        "desktop proposal {proposal_id} requires confirmation for irreversible action {irreversible_kind:?}"
    )]
    IrreversibleConfirmationRequired {
        proposal_id: String,
        irreversible_kind: IrreversibleActionKind,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    fn now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn target_id() -> DesktopTargetId {
        DesktopTargetId::new("local-demo-target")
    }

    fn target_policy() -> DesktopTargetPolicy {
        DesktopTargetPolicy {
            id: target_id(),
            platform: DesktopPlatform::Macos,
            process_name: "LocalDemoApp".to_owned(),
            executable_hash_sha256: Some("synthetic-hash".to_owned()),
            window_class: Some("SyntheticWindowClass".to_owned()),
            capability_profile: DesktopCapabilityProfile {
                profile_id: "local-demo-profile".to_owned(),
                assessed_at: now(),
                tree_completeness: AccessibilityTreeCompleteness::VerifiedComplete,
                semantic_actions: SemanticActionCapability::IrreversibleActionsWithConfirmation,
                capture_time_phi_handling: CaptureTimePhiHandling::Verified,
                visual_fallback: VisualFallbackCapability::Disabled,
            },
            authorization: DesktopAuthorization {
                authorized_by: "local-user".to_owned(),
                authorized_at: now(),
                expires_at: Some(now() + Duration::hours(1)),
                reason: "synthetic local workflow".to_owned(),
            },
            minimum_tree_completeness: AccessibilityTreeCompleteness::SemanticControlsPresent,
            allowed_observation_modes: vec![DesktopObservationMode::AccessibilityTree],
            allowed_action_classes: vec![
                DesktopActionClass::Click,
                DesktopActionClass::EnterText,
                DesktopActionClass::Submit,
            ],
            coordinate_fallback: CoordinateFallbackPolicy::Never,
        }
    }

    fn policy() -> DesktopAutomationPolicy {
        DesktopAutomationPolicy::with_targets([target_policy()])
    }

    fn observation_request(target_id: DesktopTargetId) -> DesktopObservationRequest {
        DesktopObservationRequest {
            run_id: "run-1".to_owned(),
            target_id,
            mode: DesktopObservationMode::AccessibilityTree,
            capture_policy: PhiCapturePolicy::SuppressRawCapture,
            requested_at: now(),
            redacted_reason: "synthetic observation".to_owned(),
        }
    }

    fn semantic_selector() -> ControlSelector {
        ControlSelector::Accessibility {
            control_id: "control-submit".to_owned(),
            role: ControlRole::Button,
            label_hash_sha256: Some("synthetic-label-hash".to_owned()),
        }
    }

    fn proposal(action: DesktopActionKind) -> DesktopActionProposal {
        DesktopActionProposal {
            proposal_id: "proposal-1".to_owned(),
            run_id: "run-1".to_owned(),
            target_id: target_id(),
            observation_id: "observation-1".to_owned(),
            action,
            risk: DesktopActionRisk::Moderate,
            created_at: now(),
            redacted_rationale: "synthetic action".to_owned(),
            bounds: AutomationBounds {
                max_steps: 1,
                max_wall_clock_seconds: 5,
            },
            requires_human_confirmation: false,
            verification: DesktopVerificationExpectation::NoExpectation,
        }
    }

    #[test]
    fn blocks_observation_when_target_is_not_allowlisted() {
        let request = observation_request(DesktopTargetId::new("not-allowlisted"));

        let result = policy().evaluate_observation_request(&request, now());

        assert!(matches!(
            result,
            Err(DesktopAutomationError::TargetNotAllowlisted(_))
        ));
    }

    #[test]
    fn blocks_observation_with_insufficient_tree_completeness() {
        let observation = DesktopObservation {
            observation_id: "observation-1".to_owned(),
            run_id: "run-1".to_owned(),
            target_id: target_id(),
            observed_at: now(),
            mode: DesktopObservationMode::AccessibilityTree,
            tree_completeness: AccessibilityTreeCompleteness::Partial,
            capture: DesktopCaptureSummary::phi_safe_without_raw_persistence(),
            controls: vec![],
        };

        let result = policy().evaluate_observation(&observation);

        assert!(matches!(
            result,
            Err(DesktopAutomationError::TreeCompletenessTooLow { .. })
        ));
    }

    #[test]
    fn blocks_observation_when_phi_was_not_handled_at_capture() {
        let observation = DesktopObservation {
            observation_id: "observation-1".to_owned(),
            run_id: "run-1".to_owned(),
            target_id: target_id(),
            observed_at: now(),
            mode: DesktopObservationMode::AccessibilityTree,
            tree_completeness: AccessibilityTreeCompleteness::VerifiedComplete,
            capture: DesktopCaptureSummary {
                raw_capture_persisted: false,
                phi_handled_at_capture: false,
                visual_fallback_used: false,
            },
            controls: vec![],
        };

        let result = policy().evaluate_observation(&observation);

        assert!(matches!(
            result,
            Err(DesktopAutomationError::PhiCaptureNotHandledAtCapture(_))
        ));
    }

    #[test]
    fn blocks_coordinate_action_by_default() {
        let action = DesktopActionKind::Click {
            selector: ControlSelector::Coordinates {
                rect: ScreenRect {
                    x: 840,
                    y: 210,
                    width: 80,
                    height: 20,
                },
                redacted_reason: "synthetic coordinate fallback".to_owned(),
            },
        };

        let result = policy().evaluate_proposal(&proposal(action), now());

        assert!(matches!(
            result,
            Err(DesktopAutomationError::CoordinateFallbackBlocked(_))
        ));
    }

    #[test]
    fn irreversible_action_requires_human_confirmation_flag() {
        let action = DesktopActionKind::IrreversibleCommand {
            selector: semantic_selector(),
            kind: IrreversibleActionKind::Submit,
        };

        let result = policy().evaluate_proposal(&proposal(action), now());

        assert!(matches!(
            result,
            Err(DesktopAutomationError::HumanConfirmationRequired(_))
        ));
    }

    #[test]
    fn irreversible_approval_requires_matching_confirmation() {
        let mut irreversible = proposal(DesktopActionKind::IrreversibleCommand {
            selector: semantic_selector(),
            kind: IrreversibleActionKind::Submit,
        });
        irreversible.requires_human_confirmation = true;
        let approval = DesktopActionApproval {
            approval_id: "approval-1".to_owned(),
            proposal_id: irreversible.proposal_id.clone(),
            approved_by: "local-user".to_owned(),
            approved_at: now(),
            decision: DesktopApprovalDecision::Approved,
            irreversible_confirmation: None,
        };

        let result = policy().evaluate_approval(&irreversible, &approval);

        assert!(matches!(
            result,
            Err(DesktopAutomationError::IrreversibleConfirmationRequired { .. })
        ));
    }

    #[test]
    fn accepts_allowlisted_semantic_click_with_bounds() {
        let request = observation_request(target_id());
        policy()
            .evaluate_observation_request(&request, now())
            .unwrap();

        let action = DesktopActionKind::Click {
            selector: semantic_selector(),
        };

        policy()
            .evaluate_proposal(&proposal(action), now())
            .unwrap();
    }

    #[test]
    fn authorization_expiry_blocks_target() {
        let mut target = target_policy();
        target.authorization.expires_at = Some(now() - Duration::hours(1));
        let policy = DesktopAutomationPolicy::with_targets([target]);
        let request = observation_request(target_id());

        let result = policy.evaluate_observation_request(&request, now());

        assert!(matches!(
            result,
            Err(DesktopAutomationError::AuthorizationExpired(_))
        ));
    }

    #[test]
    fn blocks_non_macos_target_in_default_scope() {
        let mut target = target_policy();
        target.platform = DesktopPlatform::Windows;
        let policy = DesktopAutomationPolicy::with_targets([target]);
        let request = observation_request(target_id());

        let result = policy.evaluate_observation_request(&request, now());

        assert!(matches!(
            result,
            Err(DesktopAutomationError::PlatformOutOfScope { .. })
        ));
    }

    #[test]
    fn blocks_action_when_capability_profile_is_observe_only() {
        let mut target = target_policy();
        target.capability_profile.semantic_actions = SemanticActionCapability::ObserveOnly;
        let policy = DesktopAutomationPolicy::with_targets([target]);
        let action = DesktopActionKind::Click {
            selector: semantic_selector(),
        };

        let result = policy.evaluate_proposal(&proposal(action), now());

        assert!(matches!(
            result,
            Err(DesktopAutomationError::CapabilityProfileBlocksAction { .. })
        ));
    }

    #[test]
    fn blocks_observation_when_capability_profile_lacks_phi_capture() {
        let mut target = target_policy();
        target.capability_profile.capture_time_phi_handling = CaptureTimePhiHandling::Unknown;
        let policy = DesktopAutomationPolicy::with_targets([target]);
        let observation = DesktopObservation {
            observation_id: "observation-1".to_owned(),
            run_id: "run-1".to_owned(),
            target_id: target_id(),
            observed_at: now(),
            mode: DesktopObservationMode::AccessibilityTree,
            tree_completeness: AccessibilityTreeCompleteness::VerifiedComplete,
            capture: DesktopCaptureSummary::phi_safe_without_raw_persistence(),
            controls: vec![],
        };

        let result = policy.evaluate_observation(&observation);

        assert!(matches!(
            result,
            Err(DesktopAutomationError::CapabilityProfileLacksPhiCapture(_))
        ));
    }
}
