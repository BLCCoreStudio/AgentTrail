use std::{env, process};

fn help() {
    println!("AgentTrail 0.1.0-dev\n\nUSAGE:\n  agenttrail status\n\nCommand recording is intentionally not enabled in this development scaffold until redaction behavior is implemented and tested.");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        help();
        return;
    }
    if args[0] == "--version" || args[0] == "-V" {
        println!("agenttrail 0.1.0-dev");
        return;
    }
    if args.len() == 1 && args[0] == "status" {
        println!("AgentTrail is in early development; persistent command recording is not enabled yet.");
        return;
    }
    eprintln!("agenttrail: unsupported command in the current development scaffold");
    process::exit(2);
}

#[cfg(test)]
mod tests {
    #[test]
    fn package_identity_is_stable() {
        assert_eq!(env!("CARGO_PKG_NAME"), "agenttrail");
    }
}
