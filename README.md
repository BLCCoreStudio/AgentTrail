# AgentTrail

**Local audit trail for explicitly wrapped development commands.**

> **Status:** development preview. No stable release has been published.

AgentTrail creates a reviewable local record for commands that are intentionally launched through its wrapper. It does **not** claim to observe every action performed by an AI agent, editor, shell, or operating system.

## Current preview

Run a command through AgentTrail:

```bash
agenttrail run -- cargo test --all-targets
```

Read the local history:

```bash
agenttrail history
```

Each event records:

- Unix timestamp
- working directory
- exit status
- wrapped command and arguments

Common secret-bearing arguments such as token/password/secret/API-key flags and inline assignments are redacted before persistence. Control characters are escaped to reduce log-injection ambiguity.

By default history is stored at:

```text
~/.local/state/agenttrail/events.log
```

Set `AGENTTRAIL_LOG` to use an explicit alternative path.

## Scope and limitations

AgentTrail only records commands launched as `agenttrail run -- ...`. It does not monitor arbitrary processes, file changes, network traffic, or commands executed outside the wrapper. Redaction is defense-in-depth, not a guarantee that every possible secret format will be recognized.

## Build

Requires Rust 1.74 or newer.

```bash
cargo build --locked
cargo test --locked
```

## Security

See [SECURITY.md](SECURITY.md).

## License

MIT © BLC Core Studio
