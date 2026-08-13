pub mod chains;

use crate::inspectors::finding::{EvidenceGraph, EvidenceNode, Finding, Relationship, Severity};

/// A mutable builder that accumulates nodes and relationships from multiple
/// inspectors, using stable namespaced IDs, then converts into an EvidenceGraph
/// or a full Finding.
///
/// Node IDs use a `"type:value"` naming convention, e.g.:
///   `"port:8080"`, `"pid:18472"`, `"exe:node"`, `"runtime:node"`, `"project:api"`
pub struct SystemGraph {
    nodes: Vec<EvidenceNode>,
    relationships: Vec<Relationship>,
}

impl SystemGraph {
    pub fn new() -> Self {
        Self { nodes: Vec::new(), relationships: Vec::new() }
    }

    /// Adds a node. Silently no-ops if a node with the same `id` already exists (idempotent).
    pub fn node(&mut self, id: &str, node_type: &str, label: &str, value: &str) {
        if !self.nodes.iter().any(|n| n.id == id) {
            self.nodes.push(EvidenceNode {
                id: id.to_string(),
                node_type: node_type.to_string(),
                label: label.to_string(),
                value: value.to_string(),
            });
        }
    }

    /// Adds a directed relationship. If either node ID doesn't exist yet, the edge is silently dropped.
    pub fn edge(&mut self, from: &str, relation: &str, to: &str) {
        let from_exists = self.nodes.iter().any(|n| n.id == from);
        let to_exists   = self.nodes.iter().any(|n| n.id == to);
        if from_exists && to_exists {
            self.relationships.push(Relationship {
                from: from.to_string(),
                relation: relation.to_string(),
                to: to.to_string(),
            });
        }
    }

    /// Merges an existing EvidenceGraph in, deduplicating nodes by ID.
    pub fn merge(&mut self, other: EvidenceGraph) {
        for n in other.nodes {
            if !self.nodes.iter().any(|existing| existing.id == n.id) {
                self.nodes.push(n);
            }
        }
        self.relationships.extend(other.relationships);
    }

    /// Consumes the builder into a validated EvidenceGraph.
    pub fn build(self) -> EvidenceGraph {
        EvidenceGraph::new(self.nodes, self.relationships)
    }

    /// Consumes the builder into a complete Finding.
    pub fn into_finding(
        self,
        severity: Severity,
        subject: impl Into<String>,
        cause: impl Into<String>,
        suggestion: Option<String>,
    ) -> Finding {
        Finding {
            severity,
            subject: subject.into(),
            cause: cause.into(),
            graph: EvidenceGraph::new(self.nodes, self.relationships),
            suggestion,
        }
    }
}
