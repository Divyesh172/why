use serde::Serialize;

#[allow(dead_code)]
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[allow(dead_code)]
impl Severity {
    pub fn to_string(&self) -> &str {
        match self {
            Severity::Info => "Info",
            Severity::Warning => "Warning",
            Severity::Error => "Error",
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct Evidence {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Finding {
    pub severity: Severity,
    pub subject: String,
    pub cause: String,
    pub evidence: Vec<Evidence>,
    pub suggestion: Option<String>,
}

impl Finding {
    /// Renders the finding to stdout as a formatted hierarchy tree branch.
    pub fn print_terminal(&self) {
        let severity_color = match self.severity {
            Severity::Info => "\x1b[32m[Info]\x1b[0m",
            Severity::Warning => "\x1b[33m[Warning]\x1b[0m",
            Severity::Error => "\x1b[31m[Error]\x1b[0m",
        };

        println!("\n\x1b[1m{} {}\x1b[0m", self.subject, severity_color);
        println!(" ├─ cause: {}", self.cause);
        
        let evidence_len = self.evidence.len();
        for (i, ev) in self.evidence.iter().enumerate() {
            let prefix = if i == evidence_len - 1 && self.suggestion.is_none() {
                " └─"
            } else {
                " ├─"
            };
            println!("{} {}: {}", prefix, ev.label, ev.value);
        }

        if let Some(ref sug) = self.suggestion {
            println!(" └─ suggestion: \x1b[36m{}\x1b[0m", sug);
        }
    }
}
