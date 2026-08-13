use std::collections::HashSet;
use serde::Serialize;
use crate::inspectors::finding::Finding;

/// A ranked, grouped collection of Findings for a single inspection target.
/// Handles severity ordering and independence analysis between findings.
#[derive(Debug, Serialize)]
pub struct DiagnosticReport {
    pub title: String,
    pub findings: Vec<Finding>,
}

impl DiagnosticReport {
    pub fn new(title: impl Into<String>, mut findings: Vec<Finding>) -> Self {
        // Sort descending: Critical > Error > Warning > Info
        findings.sort_by(|a, b| b.severity.cmp(&a.severity));
        Self { title: title.into(), findings }
    }

    pub fn add(&mut self, finding: Finding) {
        self.findings.push(finding);
        self.findings.sort_by(|a, b| b.severity.cmp(&a.severity));
    }

    /// Returns all findings whose severity is Error or Critical.
    pub fn blockers(&self) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.severity.is_blocking()).collect()
    }

    /// True if finding `a` and finding `b` share no node IDs — they are
    /// completely independent problems with separate causal chains.
    pub fn are_independent(a: &Finding, b: &Finding) -> bool {
        let a_ids: HashSet<&str> = a.graph.nodes.iter().map(|n| n.id.as_str()).collect();
        let b_ids: HashSet<&str> = b.graph.nodes.iter().map(|n| n.id.as_str()).collect();
        a_ids.is_disjoint(&b_ids)
    }

    /// Prints the full ranked report to stdout.
    pub fn print_terminal(&self) {
        let blockers = self.blockers();
        let total = self.findings.len();

        println!("\n\x1b[1m{}\x1b[0m", self.title);
        println!("{}", "─".repeat(50));

        if total == 0 {
            println!("\n\x1b[32m✓ Everything looks good.\x1b[0m\n");
            return;
        }

        // Summary line
        let blocking_count = blockers.len();
        if blocking_count > 0 {
            println!(
                "\n\x1b[1m\x1b[31m{} problem{} found\x1b[0m ({} total findings)\n",
                blocking_count,
                if blocking_count == 1 { "" } else { "s" },
                total,
            );
        } else {
            println!("\n\x1b[33m{} findings — no blocking issues\x1b[0m\n", total);
        }

        // Print all findings, ranked
        for finding in &self.findings {
            finding.print_terminal();
        }

        println!("\n{}", "─".repeat(50));

        if blockers.is_empty() {
            println!("\x1b[32m\x1b[1m✓ Environment is healthy.\x1b[0m");
        } else {
            println!("\x1b[31m\x1b[1m✗ {} blocker{} detected:\x1b[0m",
                blocking_count,
                if blocking_count == 1 { "" } else { "s" });

            for f in &blockers {
                println!("  \x1b[31m•\x1b[0m \x1b[1m{}\x1b[0m — {}", f.subject, f.cause);
                if let Some(ref s) = f.suggestion {
                    println!("      Hint: {}", s);
                }
            }

            // Independence analysis across all blocker pairs
            let independent = blockers.windows(2).all(|pair| {
                DiagnosticReport::are_independent(pair[0], pair[1])
            });

            if blockers.len() > 1 {
                if independent {
                    println!("\n  \x1b[90mThese are independent problems — fix any in any order.\x1b[0m");
                } else {
                    println!("\n  \x1b[90mSome problems may share a common root cause.\x1b[0m");
                }
            }
        }

        println!();
    }

    pub fn print_json(&self) {
        println!("{}", serde_json::to_string_pretty(self)
            .unwrap_or_else(|_| "{}".to_string()));
    }
}
