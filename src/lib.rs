//! Typed request and reply contract between `orchestrate` and the orchestrator
//! judge adapter.
//!
//! The judge decides two things: which topics a registering agent should be
//! seated on (`AssignTopic`), and how a message addressed to the orchestrator
//! should be handled (`TriageMessage`). The binary wire is rkyv-backed through
//! `signal-frame`; NOTA projection is an edge feature for clients, tests, and
//! tools.
//!
//! Transport-failure separation: the model-facing verdicts here are purely
//! semantic. Provider/configuration failures the adapter itself detects surface
//! as `RequestRejected(OrchestratorJudgeRequestRejection)`. Total judge
//! unavailability, malformed judge output, and judge timeouts are the caller's
//! concern: `orchestrate` maps them into `signal-orchestrate`'s
//! `AgentRegistrationRejected` reasons and its triage handling. They are
//! deliberately absent from `TopicAssignmentRejectionReason` and
//! `TriageRejectionReason`.

#![forbid(unsafe_code)]

use signal_orchestrate::{
    MissionDescription, OrchestratorAgentIdentifier, OrchestratorAgentSummary, OrchestratorTopic,
    OrchestratorTopicPath, TopicName,
};
use signal_orchestrator_message::{OrchestratorMessage, OrchestratorMessageKind};
use thiserror::Error;

pub const SIGNAL_SCHEMA_SOURCE: &str = include_str!("../schema/signal.schema");

pub type OrchestratorJudgeFrame =
    signal_frame::ExchangeFrame<OrchestratorJudgeRequest, OrchestratorJudgeReply>;
pub type OrchestratorJudgeFrameBody =
    signal_frame::ExchangeFrameBody<OrchestratorJudgeRequest, OrchestratorJudgeReply>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("orchestrator judge contract value is empty")]
    EmptyValue,
}

#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum OrchestratorJudgeRequest {
    AssignTopic(TopicAssignmentPacket),
    TriageMessage(TriagePacket),
}

impl signal_frame::RequestPayload for OrchestratorJudgeRequest {}

impl signal_frame::LogVariant for OrchestratorJudgeRequest {
    fn log_variant(&self) -> u64 {
        match self {
            Self::AssignTopic(_) => 1,
            Self::TriageMessage(_) => 2,
        }
    }
}

#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum OrchestratorJudgeReply {
    TopicAssigned(TopicAssignmentResponse),
    MessageTriaged(TriageResponse),
    RequestRejected(OrchestratorJudgeRequestRejection),
}

// ─── Judgment scope ───────────────────────────────────────

#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum JudgmentScope {
    Public,
    Private(PrivateJudgmentScope),
}

impl JudgmentScope {
    pub fn public() -> Self {
        Self::Public
    }

    pub fn private_hashes_and_redaction() -> Self {
        Self::Private(PrivateJudgmentScope::hashes_and_redaction())
    }
}

#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PrivateJudgmentScope {
    pub diagnostic_policy: PrivateDiagnosticPolicy,
}

impl PrivateJudgmentScope {
    pub fn new(diagnostic_policy: PrivateDiagnosticPolicy) -> Self {
        Self { diagnostic_policy }
    }

    pub fn hashes_and_redaction() -> Self {
        Self::new(PrivateDiagnosticPolicy::HashesAndRedaction)
    }
}

#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrivateDiagnosticPolicy {
    HashesAndRedaction,
}

// ─── Topic assignment ─────────────────────────────────────

#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TopicAssignmentPacket {
    pub scope: JudgmentScope,
    pub mission: MissionDescription,
    pub existing_topics: Vec<OrchestratorTopic>,
}

impl TopicAssignmentPacket {
    pub fn new(
        scope: JudgmentScope,
        mission: MissionDescription,
        existing_topics: Vec<OrchestratorTopic>,
    ) -> Self {
        Self {
            scope,
            mission,
            existing_topics,
        }
    }
}

#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TopicAssignmentResponse {
    pub verdict: TopicAssignmentVerdict,
    pub diagnostic: JudgeDiagnostic,
}

impl TopicAssignmentResponse {
    pub fn new(verdict: TopicAssignmentVerdict, diagnostic: JudgeDiagnostic) -> Self {
        Self {
            verdict,
            diagnostic,
        }
    }
}

#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum TopicAssignmentVerdict {
    Assign(TopicAssignment),
    Reject(TopicAssignmentRejectionReason),
}

/// The judge's seating decision: which existing topics to reuse, and which new
/// topics to create.
#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TopicAssignment {
    pub reuse_topics: Vec<OrchestratorTopicPath>,
    pub create_topics: Vec<NewTopic>,
}

impl TopicAssignment {
    pub fn new(reuse_topics: Vec<OrchestratorTopicPath>, create_topics: Vec<NewTopic>) -> Self {
        Self {
            reuse_topics,
            create_topics,
        }
    }
}

/// A topic the judge asks to create. `parent` is the containing topic path,
/// absent for a root topic.
#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct NewTopic {
    pub parent: Option<OrchestratorTopicPath>,
    pub name: TopicName,
}

impl NewTopic {
    pub fn new(parent: Option<OrchestratorTopicPath>, name: TopicName) -> Self {
        Self { parent, name }
    }
}

#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopicAssignmentRejectionReason {
    MissionTooVague,
    MissionEmpty,
}

// ─── Message triage ───────────────────────────────────────

#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TriagePacket {
    pub scope: JudgmentScope,
    pub incoming: OrchestratorMessage,
    pub sender: OrchestratorAgentIdentifier,
    pub topic_directory: Vec<OrchestratorTopic>,
    pub agent_directory: Vec<OrchestratorAgentSummary>,
}

impl TriagePacket {
    pub fn new(
        scope: JudgmentScope,
        incoming: OrchestratorMessage,
        sender: OrchestratorAgentIdentifier,
        topic_directory: Vec<OrchestratorTopic>,
        agent_directory: Vec<OrchestratorAgentSummary>,
    ) -> Self {
        Self {
            scope,
            incoming,
            sender,
            topic_directory,
            agent_directory,
        }
    }
}

#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TriageResponse {
    pub verdict: TriageVerdict,
    pub diagnostic: JudgeDiagnostic,
}

impl TriageResponse {
    pub fn new(verdict: TriageVerdict, diagnostic: JudgeDiagnostic) -> Self {
        Self {
            verdict,
            diagnostic,
        }
    }
}

/// The judge's triage decision. Spawning is inexpressible: there is no
/// spawn/new-session variant. A message may be routed (possibly retyped or
/// rewritten, to one or more recipients), escalated to the coordinator, or
/// rejected.
#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum TriageVerdict {
    Route(TriageRouting),
    Escalate(EscalationNote),
    Reject(TriageRejectionReason),
}

/// A routing decision. `retyped_kind` and `rewritten_message` are typed
/// positional optionals: `None` means route as-is, `Some` names the change.
#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TriageRouting {
    pub recipients: Vec<OrchestratorAgentIdentifier>,
    pub retyped_kind: Option<OrchestratorMessageKind>,
    pub rewritten_message: Option<OrchestratorMessage>,
}

impl TriageRouting {
    pub fn new(
        recipients: Vec<OrchestratorAgentIdentifier>,
        retyped_kind: Option<OrchestratorMessageKind>,
        rewritten_message: Option<OrchestratorMessage>,
    ) -> Self {
        Self {
            recipients,
            retyped_kind,
            rewritten_message,
        }
    }
}

/// An escalation to the coordinator: why it is escalated, and the detail.
#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct EscalationNote {
    pub coordinator_reason: EscalationReason,
    pub detail: EscalationDetail,
}

impl EscalationNote {
    pub fn new(coordinator_reason: EscalationReason, detail: EscalationDetail) -> Self {
        Self {
            coordinator_reason,
            detail,
        }
    }
}

#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriageRejectionReason {
    NoEligibleRecipient,
    SenderNotRegistered,
    MalformedPayload,
}

// ─── Diagnostics and request rejection ────────────────────

#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct JudgeDiagnostic {
    pub redacted_text: RedactedText,
    pub content_hashes: Vec<ContentHash>,
}

impl JudgeDiagnostic {
    pub fn new(redacted_text: RedactedText, content_hashes: Vec<ContentHash>) -> Self {
        Self {
            redacted_text,
            content_hashes,
        }
    }

    pub fn redacted(redacted_text: RedactedText) -> Self {
        Self::new(redacted_text, Vec::new())
    }
}

#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct OrchestratorJudgeRequestRejection {
    pub reason: OrchestratorJudgeRequestRejectionReason,
    pub diagnostic: JudgeDiagnostic,
}

impl OrchestratorJudgeRequestRejection {
    pub fn new(
        reason: OrchestratorJudgeRequestRejectionReason,
        diagnostic: JudgeDiagnostic,
    ) -> Self {
        Self { reason, diagnostic }
    }
}

/// Provider/configuration failures the adapter itself detects. Total judge
/// unavailability, malformed output, and timeouts are the caller's concern and
/// live in `signal-orchestrate`, not here.
#[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrchestratorJudgeRequestRejectionReason {
    InvalidRequest,
    ConfigurationUnavailable,
    ProviderUnavailable,
    ProviderRejected,
    ResponseFormatFailure,
}

macro_rules! non_empty_text_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[cfg_attr(feature = "nota-text", derive(nota::NotaDecode, nota::NotaEncode))]
        #[derive(
            rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq,
        )]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, Error> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(Error::EmptyValue);
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }
    };
}

non_empty_text_type!(ContentHash, "A content hash for privacy-safe diagnostics.");
non_empty_text_type!(RedactedText, "Redacted diagnostic text, safe to surface.");
non_empty_text_type!(
    EscalationReason,
    "Why a triage was escalated to the coordinator."
);
non_empty_text_type!(EscalationDetail, "Detail accompanying an escalation.");
