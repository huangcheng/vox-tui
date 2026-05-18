use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Text,
    Json,
}

pub struct Output {
    pub format: OutputFormat,
    pub quiet: bool,
    pub verbose: bool,
    pub no_color: bool,
}

impl Output {
    pub fn new(format: OutputFormat, quiet: bool, verbose: bool, no_color: bool) -> Self {
        Self { format, quiet, verbose, no_color }
    }

    /// Print result to stdout (respects format).
    pub fn result(&self, msg: &str) {
        println!("{msg}");
    }

    /// Print result as JSON to stdout.
    pub fn result_json(&self, value: &serde_json::Value) {
        println!("{}", serde_json::to_string_pretty(value).unwrap_or_default());
    }

    /// Print status/progress to stderr (suppressed in quiet mode).
    pub fn status(&self, msg: &str) {
        if !self.quiet && self.format == OutputFormat::Text {
            eprintln!("{msg}");
        }
    }

    /// Print error to stderr (text) or stdout (json).
    pub fn error(&self, msg: &str, code: i32) {
        match self.format {
            OutputFormat::Text => eprintln!("Error: {msg}"),
            OutputFormat::Json => {
                let json = serde_json::json!({"error": msg, "code": code});
                println!("{json}");
            }
        }
    }

    /// Print verbose debug info to stderr.
    pub fn debug(&self, msg: &str) {
        if self.verbose {
            eprintln!("[debug] {msg}");
        }
    }

    /// Print deprecation warning to stderr.
    pub fn deprecation(&self, msg: &str) {
        eprintln!("Warning: {msg}");
    }
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Self {
        match s {
            "json" => OutputFormat::Json,
            _ => OutputFormat::Text,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_from_str() {
        assert_eq!(OutputFormat::from_str("json"), OutputFormat::Json);
        assert_eq!(OutputFormat::from_str("text"), OutputFormat::Text);
        assert_eq!(OutputFormat::from_str("anything"), OutputFormat::Text);
    }
}
