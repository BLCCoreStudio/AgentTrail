# Contributing

Contributions around safe redaction, local log formats, portability, tests, and documentation are welcome.

Before opening a pull request:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

Any feature that persists command or environment information must document its privacy impact and include redaction tests. Follow `SECURITY.md` for vulnerabilities.
