# AgentTrail

**Local, reviewable change evidence for explicitly wrapped AI-assisted development commands.**

> **Status:** development preview. No stable release has been published.

AgentTrail records commands that are intentionally launched through its wrapper, creates a receipt describing the Git working-tree evidence observed before and after the command, verifies receipt integrity, and can perform a read-only review of the current working tree.

The goal is not merely to keep shell history, but to make an AI-assisted change easier to inspect and verify later.

AgentTrail does **not** claim to observe every action performed by an AI agent, editor, shell, MCP server, or operating system. It records only work explicitly launched through AgentTrail and evidence Git can expose from the current repository.

## Record a development command

```bash
agenttrail run -- cargo test --all-targets
```

For every wrapped command AgentTrail keeps the local event history and creates a receipt under:

```text
~/.local/state/agenttrail/receipts/
```

Set `AGENTTRAIL_STATE` to choose a different state directory.

## Review the current working tree

```bash
agenttrail review
```

Or inspect another repository:

```bash
agenttrail review /path/to/repository
```

The review is read-only. It reports changed entries, Git diff statistics when available, and explainable **review hints** for paths that deserve extra attention, including examples such as:

- CI/workflow configuration
- dependency/build manifests and lockfiles
- potentially sensitive path names
- authentication, security, permission, or policy-related paths

A review hint is not proof that a change is unsafe and does not prove that a specific edit was produced by an AI agent.

## What a receipt contains

The current receipt format records:

- start and finish timestamps
- working directory
- detected Git repository root
- wrapped command with common secret-bearing arguments redacted
- command exit status
- before/after Git working-tree evidence object IDs
- before/after changed-file counts
- a Git object ID over the receipt payload for later integrity verification

The working-tree evidence object is derived from Git status plus the current binary diff when available. It is intended as a compact comparison signal, not a complete forensic capture of every file byte, process, network request, or editor action.

## Verify a receipt

```bash
agenttrail verify ~/.local/state/agenttrail/receipts/<receipt>.receipt
```

Verification recalculates the Git object ID of the stored payload and fails if the receipt contents no longer match the recorded ID.

Git's object hashing algorithm depends on the repository/Git configuration; AgentTrail therefore calls this a **Git object ID**, not universally a SHA-256 digest.

## Read command history

```bash
agenttrail history
```

By default the command history is stored at:

```text
~/.local/state/agenttrail/events.log
```

Set `AGENTTRAIL_LOG` to use an explicit alternative history path.

## Secret handling

Common token/password/secret/API-key flags and inline assignments are redacted before persistence. Control characters are escaped to reduce log-injection ambiguity.

Redaction is defense-in-depth, not a guarantee that every secret format will be recognized.

## Relationship to AgentDiff

AgentDiff's useful read-only working-tree summary direction is now represented directly in AgentTrail through `agenttrail review`, including explainable risky-path review hints.

`AgentDiff` remains public as a focused companion/research history rather than being deleted or republished. Future broadly useful review behavior should integrate into AgentTrail after testing.

## Build

Requires Rust 1.74 or newer and Git.

```bash
cargo build --locked
cargo test --locked
```

## Scope and limitations

- Only commands launched as `agenttrail run -- ...` are directly recorded.
- AgentTrail does not monitor arbitrary child/grandchild activity beyond the wrapped process exit status.
- Current Git evidence does not include network traffic or a complete copy of every untracked file's contents.
- Review hints are path-based attention signals, not security verdicts.
- A verified receipt proves that the stored receipt payload matches its recorded Git object ID; it does not prove that the underlying command or resulting code is safe.

See [SECURITY.md](SECURITY.md) for reporting guidance and current limitations.

## License

MIT © BLC Core Studio
