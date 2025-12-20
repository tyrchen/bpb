//! Command-line argument parsing

use std::env;

#[derive(Debug, PartialEq)]
pub enum Command {
    Init {
        userid: String,
        identity: Option<String>,
    },
    List,
    Print {
        identity: Option<String>,
    },
    Remove {
        identity: String,
    },
    Rename {
        old_name: String,
        new_name: String,
    },
    Default {
        identity: String,
    },
    Export {
        identity: String,
        include_secret: bool,
    },
    Sign {
        identity: Option<String>,
    },
    Help,
    Delegate,
}

pub struct Args {
    pub command: Command,
}

impl Args {
    pub fn parse() -> Self {
        let args: Vec<String> = env::args().skip(1).collect();
        Args {
            command: parse_command(&args),
        }
    }
}

fn parse_command(args: &[String]) -> Command {
    if args.is_empty() {
        return Command::Delegate;
    }

    match args[0].as_str() {
        "init" => parse_init(&args[1..]),
        "list" | "ls" => Command::List,
        "print" => parse_print(&args[1..]),
        "remove" | "rm" => parse_remove(&args[1..]),
        "rename" | "mv" => parse_rename(&args[1..]),
        "default" => parse_default(&args[1..]),
        "export" => parse_export(&args[1..]),
        "--help" | "-h" | "help" => Command::Help,
        arg if is_gpg_sign_arg(arg) => parse_sign(args),
        _ => {
            // Check if any arg is a sign arg
            if args.iter().any(|a| is_gpg_sign_arg(a)) {
                parse_sign(args)
            } else {
                Command::Delegate
            }
        }
    }
}

fn parse_init(args: &[String]) -> Command {
    let mut identity = None;
    let mut userid = None;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--id" | "-i" => {
                if i + 1 < args.len() {
                    identity = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            arg if !arg.starts_with('-') => {
                userid = Some(arg.to_string());
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    match userid {
        Some(userid) => Command::Init { userid, identity },
        None => Command::Help, // Will show error in main
    }
}

fn parse_print(args: &[String]) -> Command {
    let mut identity = None;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--id" | "-i" => {
                if i + 1 < args.len() {
                    identity = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    Command::Print { identity }
}

fn parse_remove(args: &[String]) -> Command {
    if args.is_empty() {
        Command::Help
    } else {
        Command::Remove {
            identity: args[0].clone(),
        }
    }
}

fn parse_rename(args: &[String]) -> Command {
    if args.len() < 2 {
        Command::Help
    } else {
        Command::Rename {
            old_name: args[0].clone(),
            new_name: args[1].clone(),
        }
    }
}

fn parse_default(args: &[String]) -> Command {
    if args.is_empty() {
        Command::Help
    } else {
        Command::Default {
            identity: args[0].clone(),
        }
    }
}

fn parse_export(args: &[String]) -> Command {
    if args.is_empty() {
        Command::Help
    } else {
        let include_secret = args.iter().any(|a| a == "--secret" || a == "-s");
        let identity = args
            .iter()
            .find(|a| !a.starts_with('-'))
            .cloned()
            .unwrap_or_default();

        Command::Export {
            identity,
            include_secret,
        }
    }
}

fn parse_sign(args: &[String]) -> Command {
    let mut identity = None;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--id" | "-i" => {
                if i + 1 < args.len() {
                    identity = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    Command::Sign { identity }
}

fn is_gpg_sign_arg(arg: &str) -> bool {
    arg == "--sign" || (arg.starts_with('-') && !arg.starts_with("--") && arg.contains('s'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_init() {
        let args = vec!["init".to_string(), "User <user@test.com>".to_string()];
        let cmd = parse_command(&args);
        assert_eq!(
            cmd,
            Command::Init {
                userid: "User <user@test.com>".to_string(),
                identity: None,
            }
        );
    }

    #[test]
    fn test_parse_init_with_identity() {
        let args = vec![
            "init".to_string(),
            "--id".to_string(),
            "work".to_string(),
            "User <user@work.com>".to_string(),
        ];
        let cmd = parse_command(&args);
        assert_eq!(
            cmd,
            Command::Init {
                userid: "User <user@work.com>".to_string(),
                identity: Some("work".to_string()),
            }
        );
    }

    #[test]
    fn test_parse_list() {
        let args = vec!["list".to_string()];
        let cmd = parse_command(&args);
        assert_eq!(cmd, Command::List);
    }

    #[test]
    fn test_parse_print() {
        let args = vec!["print".to_string()];
        let cmd = parse_command(&args);
        assert_eq!(cmd, Command::Print { identity: None });
    }

    #[test]
    fn test_parse_print_with_identity() {
        let args = vec!["print".to_string(), "--id".to_string(), "work".to_string()];
        let cmd = parse_command(&args);
        assert_eq!(
            cmd,
            Command::Print {
                identity: Some("work".to_string())
            }
        );
    }

    #[test]
    fn test_parse_sign() {
        let args = vec!["--sign".to_string()];
        let cmd = parse_command(&args);
        assert_eq!(cmd, Command::Sign { identity: None });
    }

    #[test]
    fn test_parse_sign_with_identity() {
        let args = vec!["--sign".to_string(), "--id".to_string(), "work".to_string()];
        let cmd = parse_command(&args);
        assert_eq!(
            cmd,
            Command::Sign {
                identity: Some("work".to_string())
            }
        );
    }

    #[test]
    fn test_gpg_sign_arg() {
        assert!(is_gpg_sign_arg("--sign"));
        assert!(is_gpg_sign_arg("-s"));
        assert!(is_gpg_sign_arg("-bsau"));
        assert!(!is_gpg_sign_arg("--status-fd=2"));
        assert!(!is_gpg_sign_arg("-a"));
    }
}
