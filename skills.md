# skills — signal-orchestrator-judge

- Contract-local operation roots: `AssignTopic` and `TriageMessage`.
- Binary component traffic uses typed rkyv records. NOTA is only projection for
  clients, tests, and tools.
- Every request carries an explicit `JudgmentScope`.
- Diagnostics default to redacted text and content hashes; do not add raw private
  content fields to reply diagnostics.
- Model verdicts stay semantic; transport-failure reasons live in
  `signal-orchestrate`.
- Spawning is inexpressible in the triage verdict; keep it that way.
- Run `cargo fmt`, `cargo test`, and `nix flake check` after Rust changes.
