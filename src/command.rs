#[derive(Debug, Clone, PartialEq)]
pub enum SlashCommand {
    Provider { name: String },
    Model { name: String },
    Help,
    Clear,
    Save,
    Status,
    Unknown(String),
}

pub const COMMANDS: &[(&str, &str)] = &[
    ("provider", "Switch AI provider (e.g., /provider minimax)"),
    (
        "model",
        "Switch model within current provider (e.g., /model speech-02-hd)",
    ),
    ("help", "Show available slash commands"),
    ("clear", "Clear chat history"),
    ("save", "Save current conversation"),
    ("status", "Show current provider and model info"),
];

pub fn parse_slash_command(input: &str) -> Option<SlashCommand> {
    let input = input.trim();

    // Must start with /
    if !input.starts_with('/') {
        return None;
    }

    // // is escape for literal /
    if input.starts_with("//") {
        return None;
    }

    // Just a single / with nothing after
    if input == "/" {
        return None;
    }

    let rest = &input[1..];
    let parts: Vec<&str> = rest.splitn(2, ' ').collect();

    let cmd = parts[0].to_lowercase();
    let arg = parts.get(1).map(|s| s.trim());

    match cmd.as_str() {
        "provider" => Some(SlashCommand::Provider {
            name: arg.unwrap_or("").to_string(),
        }),
        "model" => Some(SlashCommand::Model {
            name: arg.unwrap_or("").to_string(),
        }),
        "help" => Some(SlashCommand::Help),
        "clear" => Some(SlashCommand::Clear),
        "save" => Some(SlashCommand::Save),
        "status" => Some(SlashCommand::Status),
        _ => Some(SlashCommand::Unknown(cmd)),
    }
}

pub fn complete_command(prefix: &str) -> Vec<&'static str> {
    let prefix = prefix.to_lowercase();
    COMMANDS
        .iter()
        .filter(|(name, _)| name.starts_with(&prefix))
        .map(|(name, _)| *name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_provider_command() {
        assert_eq!(
            parse_slash_command("/provider minimax"),
            Some(SlashCommand::Provider {
                name: "minimax".into()
            })
        );
    }

    #[test]
    fn test_parse_model_command() {
        assert_eq!(
            parse_slash_command("/model speech-02-hd"),
            Some(SlashCommand::Model {
                name: "speech-02-hd".into()
            })
        );
    }

    #[test]
    fn test_parse_help() {
        assert_eq!(parse_slash_command("/help"), Some(SlashCommand::Help));
    }

    #[test]
    fn test_parse_clear() {
        assert_eq!(parse_slash_command("/clear"), Some(SlashCommand::Clear));
    }

    #[test]
    fn test_parse_save() {
        assert_eq!(parse_slash_command("/save"), Some(SlashCommand::Save));
    }

    #[test]
    fn test_parse_status() {
        assert_eq!(parse_slash_command("/status"), Some(SlashCommand::Status));
    }

    #[test]
    fn test_parse_unknown() {
        assert_eq!(
            parse_slash_command("/foobar"),
            Some(SlashCommand::Unknown("foobar".into()))
        );
    }

    #[test]
    fn test_parse_not_slash() {
        assert_eq!(parse_slash_command("hello world"), None);
    }

    #[test]
    fn test_parse_double_slash_escape() {
        assert_eq!(parse_slash_command("//not a command"), None);
    }

    #[test]
    fn test_parse_empty() {
        assert_eq!(parse_slash_command(""), None);
    }

    #[test]
    fn test_parse_slash_only() {
        assert_eq!(parse_slash_command("/"), None);
    }

    #[test]
    fn test_parse_with_whitespace() {
        assert_eq!(
            parse_slash_command("  /provider   minimax  "),
            Some(SlashCommand::Provider {
                name: "minimax".into()
            })
        );
    }

    #[test]
    fn test_parse_case_insensitive() {
        assert_eq!(
            parse_slash_command("/PROVIDER minimax"),
            Some(SlashCommand::Provider {
                name: "minimax".into()
            })
        );
    }

    #[test]
    fn test_complete_provider() {
        assert_eq!(complete_command("pr"), vec!["provider"]);
    }

    #[test]
    fn test_complete_all() {
        let all: Vec<&str> = COMMANDS.iter().map(|(name, _)| *name).collect();
        assert_eq!(complete_command(""), all);
    }

    #[test]
    fn test_complete_none() {
        assert_eq!(complete_command("xyz"), Vec::<&str>::new());
    }
}
