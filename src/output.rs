use std::cell::Cell;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Text,
    Json,
}

pub struct Output {
    pub format: OutputFormat,
    pub quiet: bool,
    verbose: bool,
    #[allow(dead_code)] // reserved for future color control
    no_color: bool,
    /// Tracks the last error code set via `error()`.
    last_error_code: Cell<Option<i32>>,
}

impl Output {
    pub fn new(format: OutputFormat, quiet: bool, verbose: bool, no_color: bool) -> Self {
        Self {
            format,
            quiet,
            verbose,
            no_color,
            last_error_code: Cell::new(None),
        }
    }

    /// Print result to stdout (respects format).
    pub fn result(&self, msg: &str) {
        println!("{msg}");
    }

    /// Print result as JSON to stdout.
    #[allow(dead_code)]
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
    /// Also records the error code for later retrieval via `exit_code()`.
    pub fn error(&self, msg: &str, code: i32) {
        self.last_error_code.set(Some(code));
        match self.format {
            OutputFormat::Text => eprintln!("Error: {msg} (exit code: {code})"),
            OutputFormat::Json => {
                let json = serde_json::json!({"error": msg, "code": code});
                println!("{json}");
            }
        }
    }

    /// Print verbose debug info to stderr.
    #[allow(dead_code)]
    pub fn debug(&self, msg: &str) {
        if self.verbose {
            eprintln!("[debug] {msg}");
        }
    }

    /// Print deprecation warning to stderr.
    #[allow(dead_code)]
    pub fn deprecation(&self, msg: &str) {
        eprintln!("Warning: {msg}");
    }

    /// Check if quiet mode is enabled.
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }

    /// Returns `true` if any error has been recorded.
    pub fn has_errors(&self) -> bool {
        self.last_error_code.get().is_some()
    }

    /// Returns the exit code to use when the process terminates.
    /// Returns the last error code if an error occurred, otherwise `0`.
    pub fn exit_code(&self) -> i32 {
        self.last_error_code.get().unwrap_or(0)
    }
}

impl OutputFormat {
    #[allow(clippy::should_implement_trait)]
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
