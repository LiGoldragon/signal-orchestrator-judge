# Agent guidance — signal-orchestrator-judge

Read `ARCHITECTURE.md` before editing.

This is a contract repo. Keep it pure: typed records, binary rkyv wire shape,
NOTA projection for edges, and tests. Do not add adapter runtime, provider IO,
storage, prompt prose, or daemon logic.

Keep model verdicts semantic: do not add judge-unavailable / malformed / timeout
reasons to `TopicAssignmentRejectionReason` or `TriageRejectionReason`; those
transport failures live in `signal-orchestrate`. Do not add a spawn or
new-session triage verdict.

Run `cargo fmt`, `cargo test`, and `nix flake check` after Rust changes.
