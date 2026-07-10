# signal-orchestrator-judge — architecture

`signal-orchestrator-judge` is the contract between `orchestrate` and the
orchestrator judge text/model edge adapter.

It owns both sides of the exchange: `orchestrate` sends typed judge requests, and
the adapter returns typed judge replies. The surface covers two judge families:
topic assignment for registration, and message triage.

## Boundary

Owned here:

- `OrchestratorJudgeRequest` and `OrchestratorJudgeReply`;
- topic-assignment and triage packet records;
- explicit public/private request scope records;
- typed verdict, rejection, and privacy-safe diagnostic records;
- rkyv-compatible wire records and NOTA projection for edges.

Not owned here:

- provider calls and retries, which belong in the judge adapter;
- prompt prose, which belongs in an orchestrator judge configuration repo;
- adapter process lifecycle;
- topic-tree and agent-registry storage, minted identity, reachability
  discovery, and thread minting, which belong in `orchestrate` and the message
  daemon.

## Transport-failure separation

Model-facing verdicts are purely semantic. `TopicAssignmentRejectionReason` is
`MissionTooVague`/`MissionEmpty`; `TriageRejectionReason` is
`NoEligibleRecipient`/`SenderNotRegistered`/`MalformedPayload`. Provider and
configuration failures the adapter detects surface as
`RequestRejected(OrchestratorJudgeRequestRejection)`. Total judge unavailability,
malformed output, and timeouts are the caller's concern and live in
`signal-orchestrate`'s registration rejection and `orchestrate`'s triage
handling — deliberately not in this crate's model verdicts.

## Spawning is inexpressible

The triage verdict admits routing (as-is, retyped, rewritten, fanned out),
escalation to the coordinator, or rejection. There is no spawn or new-session
variant.

## Privacy

Requests name whether the adapter may receive public or private content. Replies
and diagnostics stay privacy-safe by default: redacted messages and content
hashes, not raw private content.
