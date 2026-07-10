//! Round-trip tests for the orchestrator judge contract.
//!
//! Every request, reply, verdict, and reason variant round-trips through the
//! binary `signal-frame` exchange. The NOTA projection is exercised under the
//! `nota-text` feature.

use signal_frame::{
    ExchangeFrameBody, ExchangeIdentifier, ExchangeLane, LaneSequence, NonEmpty, Reply, Request,
    SessionEpoch, ShortHeader, SubReply,
};
use signal_orchestrate::{
    MissionDescription, OrchestratorAgentIdentifier, OrchestratorAgentStatus,
    OrchestratorAgentSummary, OrchestratorTopic, OrchestratorTopicPath, TopicName,
};
use signal_orchestrator_judge::{
    ContentHash, EscalationDetail, EscalationNote, EscalationReason, JudgeDiagnostic,
    JudgmentScope, NewTopic, OrchestratorJudgeFrame, OrchestratorJudgeReply,
    OrchestratorJudgeRequest, OrchestratorJudgeRequestRejection,
    OrchestratorJudgeRequestRejectionReason, RedactedText, TopicAssignment, TopicAssignmentPacket,
    TopicAssignmentRejectionReason, TopicAssignmentResponse, TopicAssignmentVerdict, TriagePacket,
    TriageRejectionReason, TriageResponse, TriageRouting, TriageVerdict,
};
use signal_orchestrator_message::{
    GuidanceMagnitude, MessageContent, MessageSubject, OrchestratorMessage, OrchestratorMessageKind,
};

fn exchange_identifier() -> ExchangeIdentifier {
    ExchangeIdentifier::new(
        SessionEpoch::new(7),
        ExchangeLane::Connector,
        LaneSequence::first(),
    )
}

fn mission() -> MissionDescription {
    MissionDescription::from_text("Design the orchestrator messaging wire contracts.")
        .expect("mission")
}

fn topic_path(path: &str) -> OrchestratorTopicPath {
    OrchestratorTopicPath::from_wire_token(path).expect("topic path")
}

fn topic(path: &str, name: &str, parent: Option<&str>) -> OrchestratorTopic {
    OrchestratorTopic {
        path: topic_path(path),
        name: TopicName::from_text(name).expect("topic name"),
        parent: parent.map(topic_path),
    }
}

fn agent_identifier(token: &str) -> OrchestratorAgentIdentifier {
    OrchestratorAgentIdentifier::from_wire_token(token).expect("agent identifier")
}

fn agent_summary(token: &str) -> OrchestratorAgentSummary {
    OrchestratorAgentSummary {
        agent_identifier: agent_identifier(token),
        mission: mission(),
        topics: vec![topic_path("contracts")],
        status: OrchestratorAgentStatus::Active,
    }
}

fn message() -> OrchestratorMessage {
    OrchestratorMessage::new(
        OrchestratorMessageKind::Guidance(GuidanceMagnitude::Standard),
        MessageSubject::new("rebase before landing").expect("subject"),
        MessageContent::new("Fold this in at your next natural turn.").expect("content"),
    )
}

fn diagnostic() -> JudgeDiagnostic {
    JudgeDiagnostic::new(
        RedactedText::new("private details redacted").expect("redacted"),
        vec![ContentHash::new("sha256:fixture-content-hash").expect("hash")],
    )
}

fn topic_assignment_packet() -> TopicAssignmentPacket {
    TopicAssignmentPacket::new(
        JudgmentScope::public(),
        mission(),
        vec![topic("contracts", "Contracts", None)],
    )
}

fn triage_packet() -> TriagePacket {
    TriagePacket::new(
        JudgmentScope::private_hashes_and_redaction(),
        message(),
        agent_identifier("4zqk"),
        vec![topic("contracts", "Contracts", None)],
        vec![agent_summary("7mtp")],
    )
}

fn round_trips_as_request(request: OrchestratorJudgeRequest) {
    let frame = OrchestratorJudgeFrame::with_short_header(
        ShortHeader::new(1),
        ExchangeFrameBody::Request {
            exchange: exchange_identifier(),
            request: Request::from_payload(request),
        },
    );
    let encoded = frame.encode_length_prefixed().expect("encode");
    let decoded = OrchestratorJudgeFrame::decode_length_prefixed(&encoded).expect("decode");
    assert_eq!(decoded.body(), frame.body());
}

fn round_trips_as_reply(reply: OrchestratorJudgeReply) {
    let frame = OrchestratorJudgeFrame::with_short_header(
        ShortHeader::new(1),
        ExchangeFrameBody::Reply {
            exchange: exchange_identifier(),
            reply: Reply::committed(NonEmpty::single(SubReply::Ok(reply))),
        },
    );
    let encoded = frame.encode_length_prefixed().expect("encode");
    let decoded = OrchestratorJudgeFrame::decode_length_prefixed(&encoded).expect("decode");
    assert_eq!(decoded.body(), frame.body());
}

#[test]
fn private_scope_names_hashes_and_redaction_policy() {
    assert!(matches!(
        JudgmentScope::private_hashes_and_redaction(),
        JudgmentScope::Private(_)
    ));
}

#[test]
fn assign_topic_request_round_trips() {
    round_trips_as_request(OrchestratorJudgeRequest::AssignTopic(
        topic_assignment_packet(),
    ));
}

#[test]
fn triage_message_request_round_trips() {
    round_trips_as_request(OrchestratorJudgeRequest::TriageMessage(triage_packet()));
}

#[test]
fn topic_assigned_reply_round_trips_for_assign_and_every_reject_reason() {
    let assign = OrchestratorJudgeReply::TopicAssigned(TopicAssignmentResponse::new(
        TopicAssignmentVerdict::Assign(TopicAssignment::new(
            vec![topic_path("contracts")],
            vec![NewTopic::new(
                Some(topic_path("contracts")),
                TopicName::from_text("Wire").expect("name"),
            )],
        )),
        diagnostic(),
    ));
    round_trips_as_reply(assign);

    for reason in [
        TopicAssignmentRejectionReason::MissionTooVague,
        TopicAssignmentRejectionReason::MissionEmpty,
    ] {
        round_trips_as_reply(OrchestratorJudgeReply::TopicAssigned(
            TopicAssignmentResponse::new(TopicAssignmentVerdict::Reject(reason), diagnostic()),
        ));
    }
}

#[test]
fn message_triaged_reply_round_trips_for_route_escalate_and_every_reject_reason() {
    let route = OrchestratorJudgeReply::MessageTriaged(TriageResponse::new(
        TriageVerdict::Route(TriageRouting::new(
            vec![agent_identifier("7mtp")],
            Some(OrchestratorMessageKind::Report),
            Some(message()),
        )),
        diagnostic(),
    ));
    round_trips_as_reply(route);

    let route_as_is = OrchestratorJudgeReply::MessageTriaged(TriageResponse::new(
        TriageVerdict::Route(TriageRouting::new(
            vec![agent_identifier("7mtp")],
            None,
            None,
        )),
        diagnostic(),
    ));
    round_trips_as_reply(route_as_is);

    let escalate = OrchestratorJudgeReply::MessageTriaged(TriageResponse::new(
        TriageVerdict::Escalate(EscalationNote::new(
            EscalationReason::new("no seated owner for this topic").expect("reason"),
            EscalationDetail::new("coordinator should assign an owner").expect("detail"),
        )),
        diagnostic(),
    ));
    round_trips_as_reply(escalate);

    for reason in [
        TriageRejectionReason::NoEligibleRecipient,
        TriageRejectionReason::SenderNotRegistered,
        TriageRejectionReason::MalformedPayload,
    ] {
        round_trips_as_reply(OrchestratorJudgeReply::MessageTriaged(TriageResponse::new(
            TriageVerdict::Reject(reason),
            diagnostic(),
        )));
    }
}

#[test]
fn request_rejected_reply_round_trips_for_every_reason() {
    for reason in [
        OrchestratorJudgeRequestRejectionReason::InvalidRequest,
        OrchestratorJudgeRequestRejectionReason::ConfigurationUnavailable,
        OrchestratorJudgeRequestRejectionReason::ProviderUnavailable,
        OrchestratorJudgeRequestRejectionReason::ProviderRejected,
        OrchestratorJudgeRequestRejectionReason::ResponseFormatFailure,
    ] {
        round_trips_as_reply(OrchestratorJudgeReply::RequestRejected(
            OrchestratorJudgeRequestRejection::new(reason, diagnostic()),
        ));
    }
}

#[cfg(feature = "nota-text")]
#[test]
fn nota_projection_names_triage_route_shape() {
    use nota::NotaEncode;

    let reply = OrchestratorJudgeReply::MessageTriaged(TriageResponse::new(
        TriageVerdict::Route(TriageRouting::new(
            vec![agent_identifier("7mtp")],
            None,
            None,
        )),
        JudgeDiagnostic::redacted(RedactedText::new("routed").expect("redacted")),
    ));
    let text = reply.to_nota();
    assert!(text.contains("MessageTriaged"));
    assert!(text.contains("Route"));
}
