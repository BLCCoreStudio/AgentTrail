use std::{
    env, fs,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    process::{self, Command},
    time::{SystemTime, UNIX_EPOCH},
};

const REDACTED: &str = "[REDACTED]";

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

fn log_path() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("AGENTTRAIL_LOG") {
        return Ok(PathBuf::from(path));
    }

    let home = env::var_os("HOME").ok_or_else(|| {
        "HOME is not set; set AGENTTRAIL_LOG to choose an explicit log path".to_owned()
    })?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("state")
        .join("agenttrail")
        .join("events.log"))
}

fn append_event(command: &[String], exit: &str) -> Result<PathBuf, String> {
    let path = log_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock is before Unix epoch: {error}"))?
        .as_secs();
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

fn run_wrapped(command: &[String]) -> i32 {
    if command.is_empty() {
        eprintln!("agenttrail: no command supplied after '--'");
        return 2;
    }

    let status = match Command::new(&command[0]).args(&command[1..]).status() {
        Ok(status) => status,
        Err(error) => {
            let _ = append_event(command, "spawn-error");
            eprintln!("agenttrail: failed to execute '{}': {error}", command[0]);
            return 2;
        }
    };

    let code = status.code().unwrap_or(1);
    if let Err(error) = append_event(command, &code.to_string()) {
        eprintln!("agenttrail: command finished, but audit logging failed: {error}");
        return 2;
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
        "AgentTrail 0.1.0-dev\n\nUSAGE:\n  agenttrail run -- <COMMAND> [ARGS...]\n  agenttrail history\n\nOnly commands explicitly launched through AgentTrail are recorded. Logs stay local and common secret-bearing arguments are redacted before persistence."
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || matches!(args[0].as_str(), "--help" | "-h") {
        help();
        return;
    }
    if matches!(args[0].as_str(), "--version" | "-V") {
        println!("agenttrail 0.1.0-dev");
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
        let args = vec!["curl".to_owned(), "--token".to_owned(), "secret-value".to_owned()];
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
