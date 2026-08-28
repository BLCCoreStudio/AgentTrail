# Security Policy

AgentTrail is under active development and should be treated as a development-preview audit helper, not a complete monitoring or security boundary.

The current implementation records only commands explicitly launched through `agenttrail run -- ...`. Audit events are stored locally and common secret-bearing command-line arguments are redacted before persistence. This redaction is defense-in-depth and cannot guarantee recognition of every possible secret format.

AgentTrail does not monitor commands executed outside its wrapper, arbitrary processes, file changes, network traffic, editor actions, or operating-system activity.

Please report suspected vulnerabilities privately through GitHub private vulnerability reporting when available or another appropriate private channel. Include reproduction steps and impact, but never attach real credentials, tokens, private keys, or other sensitive data.
