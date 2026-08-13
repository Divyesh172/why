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
pub struct EvidenceNode {
    pub id: String,
    pub node_type: String, // e.g. "File", "Constraint", "Runtime", "Binary", "Service", "Port"
    pub label: String,
    pub value: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Relationship {
    pub from: String,
    pub relation: String,
    pub to: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Finding {
    pub severity: Severity,
    pub subject: String,
    pub cause: String,
    pub nodes: Vec<EvidenceNode>,
    pub relationships: Vec<Relationship>,
    pub suggestion: Option<String>,
}

impl Finding {
    /// Walk and render the evidence graph relationships topologically as a sequential flow chart.
    pub fn print_terminal(&self) {
        let severity_color = match self.severity {
            Severity::Info => "\x1b[32m[Info]\x1b[0m",
            Severity::Warning => "\x1b[33m[Warning]\x1b[0m",
            Severity::Error => "\x1b[31m[Error]\x1b[0m",
        };

        println!("\n\x1b[1m{} {}\x1b[0m", self.subject, severity_color);
        println!("  Cause: {}", self.cause);
        
        println!("  Evidence Graph:");
        let mut printed_nodes = std::collections::HashSet::new();

        // 1. Identify all source nodes (no incoming relationships) and trace forward
        for node in &self.nodes {
            let has_incoming = self.relationships.iter().any(|r| r.to == node.id);
            if !has_incoming {
                let mut current_id = node.id.clone();
                let val_suffix = if node.value.is_empty() {
                    "".to_string()
                } else {
                    format!(" ({})", node.value)
                };
                println!("    {} [{}]{}", node.label, node.node_type, val_suffix);
                printed_nodes.insert(current_id.clone());
                
                // Follow the outgoing chain
                while let Some(rel) = self.relationships.iter().find(|r| r.from == current_id) {
                    if let Some(next_node) = self.nodes.iter().find(|n| n.id == rel.to) {
                        println!("      \x1b[35m↓ {}\x1b[0m", rel.relation);
                        let next_val_suffix = if next_node.value.is_empty() {
                            "".to_string()
                        } else {
                            format!(" ({})", next_node.value)
                        };
                        println!("    {} [{}]{}", next_node.label, next_node.node_type, next_val_suffix);
                        current_id = next_node.id.clone();
                        printed_nodes.insert(current_id.clone());
                    } else {
                        break;
                    }
                }
            }
        }
        
        // 2. Print any remaining disconnected nodes
        for node in &self.nodes {
            if !printed_nodes.contains(&node.id) {
                let val_suffix = if node.value.is_empty() {
                    "".to_string()
                } else {
                    format!(" ({})", node.value)
                };
                println!("    {} [{}]{}", node.label, node.node_type, val_suffix);
            }
        }

        if let Some(ref sug) = self.suggestion {
            println!("  Suggestion: \x1b[36m{}\x1b[0m", sug);
        }
    }
}
