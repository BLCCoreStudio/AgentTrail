use std::{
    env, fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    process::{self, Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

const REDACTED: &str = "[REDACTED]";

#[derive(Debug)]
struct Snapshot {
    repository: String,
    evidence_oid: String,
    changed_files: usize,
}

fn is_sensitive_flag(value: &str) -> bool {
    let normalized = value
        .trim_start_matches('-')
        .replace('-', "_")
        .to_ascii_lowercase();
    [
        "token",
        "password",
        "passwd",
        "secret",
        "api_key",
        "apikey",
        "authorization",
        "credential",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn redact_inline(value: &str) -> String {
    if let Some((key, _)) = value.split_once('=') {
        if is_sensitive_flag(key) {
            return format!("{key}={REDACTED}");
        }
    }

    if value.to_ascii_lowercase().starts_with("bearer ") {
        return format!("Bearer {REDACTED}");
    }

    value.to_owned()
}

fn redact_args(args: &[String]) -> Vec<String> {
    let mut output = Vec::with_capacity(args.len());
    let mut redact_next = false;

    for arg in args {
        if redact_next {
            output.push(REDACTED.to_owned());
            redact_next = false;
            continue;
        }

        let redacted = redact_inline(arg);
        let inline_changed = redacted != *arg;
        output.push(redacted);
        if !inline_changed && is_sensitive_flag(arg) {
            redact_next = true;
        }
    }

    output
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}

fn unix_timestamp() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| format!("system clock is before Unix epoch: {error}"))
}

fn state_dir() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("AGENTTRAIL_STATE") {
        return Ok(PathBuf::from(path));
    }

    let home = env::var_os("HOME").ok_or_else(|| {
        "HOME is not set; set AGENTTRAIL_STATE to choose an explicit state directory".to_owned()
    })?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("state")
        .join("agenttrail"))
}

fn log_path() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("AGENTTRAIL_LOG") {
        return Ok(PathBuf::from(path));
    }
    Ok(state_dir()?.join("events.log"))
}

fn append_event(command: &[String], exit: &str) -> Result<PathBuf, String> {
    let path = log_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }

    let timestamp = unix_timestamp()?;
    let cwd = env::current_dir().map_err(|error| format!("failed to read cwd: {error}"))?;
    let redacted = redact_args(command)
        .iter()
        .map(|value| escape_field(value))
        .collect::<Vec<_>>()
        .join(" ");

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|error| format!("failed to open '{}': {error}", path.display()))?;

    writeln!(
        file,
        "ts={timestamp}\tcwd={}\texit={}\tcommand={}",
        escape_field(&cwd.display().to_string()),
        escape_field(exit),
        redacted
    )
    .map_err(|error| format!("failed to write '{}': {error}", path.display()))?;

    Ok(path)
}

fn command_output(args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git").args(args).output().ok()?;
    output.status.success().then_some(output.stdout)
}

fn git_hash(payload: &[u8]) -> Result<String, String> {
    let mut child = Command::new("git")
        .args(["hash-object", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to launch git hash-object: {error}"))?;

    child
        .stdin
        .as_mut()
        .ok_or_else(|| "failed to open git hash-object stdin".to_owned())?
        .write_all(payload)
        .map_err(|error| format!("failed to send receipt payload to git hash-object: {error}"))?;

    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed to read git hash-object result: {error}"))?;
    if !output.status.success() {
        return Err("git hash-object failed".to_owned());
    }

    let oid = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if oid.is_empty() {
        return Err("git hash-object returned an empty object id".to_owned());
    }
    Ok(oid)
}

fn capture_snapshot() -> Snapshot {
    let repository = command_output(&["rev-parse", "--show-toplevel"])
        .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_owned())
        .filter(|value| !value.is_empty());

    let Some(repository) = repository else {
        return Snapshot {
            repository: "not-a-git-repository".to_owned(),
            evidence_oid: "unavailable".to_owned(),
            changed_files: 0,
        };
    };

    let status = command_output(&["status", "--porcelain=v1", "--untracked-files=all"])
        .unwrap_or_default();
    let diff = command_output(&["diff", "--binary", "HEAD", "--"])
        .or_else(|| command_output(&["diff", "--binary", "--"]))
        .unwrap_or_default();

    let changed_files = String::from_utf8_lossy(&status)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();

    let mut evidence = Vec::with_capacity(status.len() + diff.len() + 32);
    evidence.extend_from_slice(b"STATUS\n");
    evidence.extend_from_slice(&status);
    evidence.extend_from_slice(b"\nDIFF\n");
    evidence.extend_from_slice(&diff);

    let evidence_oid = git_hash(&evidence).unwrap_or_else(|_| "unavailable".to_owned());

    Snapshot {
        repository,
        evidence_oid,
        changed_files,
    }
}

fn receipt_path(started: u64) -> Result<PathBuf, String> {
    let dir = state_dir()?.join("receipts");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create '{}': {error}", dir.display()))?;
    Ok(dir.join(format!("{started}-{}.receipt", process::id())))
}

fn write_receipt(
    command: &[String],
    exit: &str,
    started: u64,
    finished: u64,
    before: &Snapshot,
    after: &Snapshot,
) -> Result<PathBuf, String> {
    let path = receipt_path(started)?;
    let cwd = env::current_dir().map_err(|error| format!("failed to read cwd: {error}"))?;
    let redacted = redact_args(command)
        .iter()
        .map(|value| escape_field(value))
        .collect::<Vec<_>>()
        .join(" ");

    let payload = format!(
        "agenttrail_receipt=1\nstarted_ts={started}\nfinished_ts={finished}\ncwd={}\nrepository={}\nexit={}\ncommand={}\nbefore_evidence_oid={}\nafter_evidence_oid={}\nbefore_changed_files={}\nafter_changed_files={}\n",
        escape_field(&cwd.display().to_string()),
        escape_field(&after.repository),
        escape_field(exit),
        redacted,
        escape_field(&before.evidence_oid),
        escape_field(&after.evidence_oid),
        before.changed_files,
        after.changed_files,
    );
    let oid = git_hash(payload.as_bytes())?;
    let content = format!("{payload}payload_git_oid={oid}\n");
    fs::write(&path, content)
        .map_err(|error| format!("failed to write '{}': {error}", path.display()))?;
    Ok(path)
}

fn verify_receipt(path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    let trimmed = content.trim_end_matches(['\r', '\n']);
    let marker = "\npayload_git_oid=";
    let (payload_without_newline, expected) = trimmed
        .rsplit_once(marker)
        .ok_or_else(|| "receipt is missing payload_git_oid".to_owned())?;
    let payload = format!("{payload_without_newline}\n");
    let actual = git_hash(payload.as_bytes())?;
    if actual != expected {
        return Err(format!(
            "receipt verification failed: expected {expected}, calculated {actual}"
        ));
    }
    println!("VERIFIED: {} ({actual})", path.display());
    Ok(())
}

fn run_wrapped(command: &[String]) -> i32 {
    if command.is_empty() {
        eprintln!("agenttrail: no command supplied after '--'");
        return 2;
    }

    let started = match unix_timestamp() {
        Ok(value) => value,
        Err(error) => {
            eprintln!("agenttrail: {error}");
            return 2;
        }
    };
    let before = capture_snapshot();

    let status = match Command::new(&command[0]).args(&command[1..]).status() {
        Ok(status) => status,
        Err(error) => {
            let _ = append_event(command, "spawn-error");
            eprintln!("agenttrail: failed to execute '{}': {error}", command[0]);
            return 2;
        }
    };

    let code = status.code().unwrap_or(1);
    let exit = code.to_string();
    if let Err(error) = append_event(command, &exit) {
        eprintln!("agenttrail: command finished, but audit logging failed: {error}");
        return 2;
    }

    let after = capture_snapshot();
    let finished = unix_timestamp().unwrap_or(started);
    match write_receipt(command, &exit, started, finished, &before, &after) {
        Ok(path) => {
            println!("AgentTrail receipt: {}", path.display());
            if before.evidence_oid != after.evidence_oid {
                println!(
                    "Working-tree evidence changed: {} -> {}; changed files: {} -> {}",
                    before.evidence_oid,
                    after.evidence_oid,
                    before.changed_files,
                    after.changed_files
                );
            }
        }
        Err(error) => {
            eprintln!("agenttrail: command finished, but receipt creation failed: {error}");
            return 2;
        }
    }

    code
}

fn print_history() -> Result<(), String> {
    let path = log_path()?;
    match fs::read_to_string(&path) {
        Ok(content) => {
            print!("{content}");
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            println!("No AgentTrail history yet.");
            Ok(())
        }
        Err(error) => Err(format!("failed to read '{}': {error}", path.display())),
    }
}

fn help() {
    println!(
        "AgentTrail 0.2.0-dev\n\nUSAGE:\n  agenttrail run -- <COMMAND> [ARGS...]\n  agenttrail history\n  agenttrail verify <RECEIPT>\n\nAgentTrail records explicitly wrapped commands and now creates a local change-evidence receipt containing before/after Git working-tree evidence object IDs, changed-file counts, timing, exit status, and a redacted command. Receipts are integrity-checked with the repository's Git object hashing mechanism."
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || matches!(args[0].as_str(), "--help" | "-h") {
        help();
        return;
    }
    if matches!(args[0].as_str(), "--version" | "-V") {
        println!("agenttrail 0.2.0-dev");
        return;
    }

    match args[0].as_str() {
        "run" => {
            if args.get(1).map(String::as_str) != Some("--") {
                eprintln!("agenttrail: expected 'run -- <COMMAND> [ARGS...]'");
                process::exit(2);
            }
            process::exit(run_wrapped(&args[2..]));
        }
        "history" if args.len() == 1 => {
            if let Err(error) = print_history() {
                eprintln!("agenttrail: {error}");
                process::exit(2);
            }
        }
        "verify" if args.len() == 2 => {
            if let Err(error) = verify_receipt(Path::new(&args[1])) {
                eprintln!("agenttrail: {error}");
                process::exit(3);
            }
        }
        _ => {
            eprintln!("agenttrail: unsupported command; use --help");
            process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{escape_field, redact_args, REDACTED};

    #[test]
    fn redacts_value_after_sensitive_flag() {
        let args = vec![
            "curl".to_owned(),
            "--token".to_owned(),
            "secret-value".to_owned(),
        ];
        let redacted = redact_args(&args);
        assert_eq!(redacted[2], REDACTED);
    }

    #[test]
    fn redacts_inline_assignment() {
        let args = vec!["API_KEY=secret-value".to_owned()];
        assert_eq!(redact_args(&args), vec!["API_KEY=[REDACTED]"]);
    }

    #[test]
    fn escapes_log_control_characters() {
        assert_eq!(escape_field("a\tb\nc"), "a\\tb\\nc");
    }
}
