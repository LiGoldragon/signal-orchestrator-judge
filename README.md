# signal-orchestrator-judge

Typed Signal contract between `orchestrate` and the orchestrator judge adapter.

The contract owns both sides of two exchanges: topic assignment (`AssignTopic`),
which seats a registering agent on topics from its mission, and message triage
(`TriageMessage`), which decides how a message addressed to the orchestrator is
handled. Spawning is inexpressible in the triage verdict.

The binary wire is rkyv-backed through `signal-frame`; NOTA projection is only
for clients, tests, and tools. Topic and agent vocabulary is imported from
`signal-orchestrate`; the message payload from `signal-orchestrator-message`.
