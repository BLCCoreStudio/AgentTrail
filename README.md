# AgentTrail

**Local audit trail for commands, file changes, and actions performed by AI coding agents.**

> **Status:** early development. No stable release has been published.

AgentTrail is intended to create a reviewable local record of agent-assisted development activity without requiring a remote telemetry service.

## Planned v0.1

- record explicitly wrapped command executions
- capture timestamps, exit status, and working directory
- redact common secret-bearing arguments before persistence
- keep logs local by default
- provide human-readable history and machine-readable export
- document clearly what AgentTrail can and cannot observe

The current repository contains the development scaffold only. Command recording is not implemented yet because logging command arguments safely requires a deliberate redaction model.

## Build

Requires Rust 1.74 or newer.

```bash
cargo build
cargo test
```

## Security

See [SECURITY.md](SECURITY.md).

## License

MIT © BLC Core Studio
