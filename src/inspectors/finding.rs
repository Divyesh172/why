use serde::Serialize;
use std::collections::{HashSet, VecDeque};

#[allow(dead_code)]
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[allow(dead_code)]
impl Severity {
    pub fn label(&self) -> &str {
        match self {
            Severity::Info => "Info",
            Severity::Warning => "Warning",
            Severity::Error => "Error",
        }
    }
}

#[derive(Debug, Clone)]
pub enum GraphValidationError {
    DuplicateNodeId(String),
    MissingNodeReference(String),
    DuplicateRelationship(String, String, String),
    CycleDetected(String),
}

impl std::fmt::Display for GraphValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphValidationError::DuplicateNodeId(id) =>
                write!(f, "Duplicate node ID: {}", id),
            GraphValidationError::MissingNodeReference(id) =>
                write!(f, "Relationship references missing node ID: {}", id),
            GraphValidationError::DuplicateRelationship(from, rel, to) =>
                write!(f, "Duplicate relationship: {} -[{}]-> {}", from, rel, to),
            GraphValidationError::CycleDetected(id) =>
                write!(f, "Cycle detected at node ID: {}", id),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct EvidenceNode {
    pub id: String,
    /// Semantic category: "File", "Constraint", "Runtime", "Binary", "Service", "Port", etc.
    pub node_type: String,
    pub label: String,
    pub value: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Relationship {
    pub from: String,
    pub relation: String,
    pub to: String,
}

/// A validated, serialisable directed acyclic graph of causal evidence.
#[derive(Debug, Serialize, Clone)]
pub struct EvidenceGraph {
    pub nodes: Vec<EvidenceNode>,
    pub relationships: Vec<Relationship>,
}

impl EvidenceGraph {
    pub fn new(nodes: Vec<EvidenceNode>, relationships: Vec<Relationship>) -> Self {
        Self { nodes, relationships }
    }

    /// Validates structural constraints: unique IDs, valid references, no duplicates, no cycles.
    pub fn validate(&self) -> Result<(), GraphValidationError> {
        // 1. Unique node IDs
        let mut node_ids: HashSet<&String> = HashSet::new();
        for node in &self.nodes {
            if !node_ids.insert(&node.id) {
                return Err(GraphValidationError::DuplicateNodeId(node.id.clone()));
            }
        }

        // 2. All relationship references must point to existing nodes; no duplicate edges
        let mut rels_seen: HashSet<(String, String, String)> = HashSet::new();
        for rel in &self.relationships {
            if !node_ids.contains(&rel.from) {
                return Err(GraphValidationError::MissingNodeReference(rel.from.clone()));
            }
            if !node_ids.contains(&rel.to) {
                return Err(GraphValidationError::MissingNodeReference(rel.to.clone()));
            }
            let key = (rel.from.clone(), rel.relation.clone(), rel.to.clone());
            if !rels_seen.insert(key) {
                return Err(GraphValidationError::DuplicateRelationship(
                    rel.from.clone(), rel.relation.clone(), rel.to.clone(),
                ));
            }
        }

        // 3. Cycle detection via DFS with a recursion stack
        let mut visited: HashSet<String> = HashSet::new();
        let mut rec_stack: HashSet<String> = HashSet::new();
        for node in &self.nodes {
            if self.detect_cycle(&node.id, &mut visited, &mut rec_stack) {
                return Err(GraphValidationError::CycleDetected(node.id.clone()));
            }
        }

        Ok(())
    }

    fn detect_cycle(
        &self,
        id: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        if rec_stack.contains(id) { return true; }
        if visited.contains(id)   { return false; }

        visited.insert(id.to_string());
        rec_stack.insert(id.to_string());

        for rel in &self.relationships {
            if rel.from == id && self.detect_cycle(&rel.to, visited, rec_stack) {
                return true;
            }
        }

        rec_stack.remove(id);
        false
    }

    /// Returns all nodes with no incoming relationships (DAG sources / roots).
    pub fn roots(&self) -> Vec<&EvidenceNode> {
        self.nodes.iter()
            .filter(|n| !self.relationships.iter().any(|r| r.to == n.id))
            .collect()
    }

    /// Returns every (relationship, target_node) pair reachable from `parent_id` in one step.
    pub fn children(&self, parent_id: &str) -> Vec<(&Relationship, &EvidenceNode)> {
        self.relationships.iter()
            .filter(|r| r.from == parent_id)
            .filter_map(|r| {
                self.nodes.iter().find(|n| n.id == r.to).map(|n| (r, n))
            })
            .collect()
    }

    /// BFS path search. Returns the list of (from, relation, to) triples from `from` to `to`,
    /// or `None` if no path exists.
    pub fn trace(&self, from: &str, to: &str) -> Option<Vec<(String, String, String)>> {
        if from == to { return Some(vec![]); }

        let mut queue: VecDeque<(String, Vec<(String, String, String)>)> = VecDeque::new();
        queue.push_back((from.to_string(), vec![]));

        let mut visited: HashSet<String> = HashSet::new();
        visited.insert(from.to_string());

        while let Some((current, path)) = queue.pop_front() {
            for rel in &self.relationships {
                if rel.from == current && !visited.contains(&rel.to) {
                    let mut new_path = path.clone();
                    new_path.push((rel.from.clone(), rel.relation.clone(), rel.to.clone()));

                    if rel.to == to {
                        return Some(new_path);
                    }

                    visited.insert(rel.to.clone());
                    queue.push_back((rel.to.clone(), new_path));
                }
            }
        }

        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Finding: a validated graph with severity metadata and a suggestion
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct Finding {
    pub severity: Severity,
    pub subject: String,
    pub cause: String,
    pub graph: EvidenceGraph,
    pub suggestion: Option<String>,
}

impl Finding {
    /// Pretty-prints the Finding to stdout, rendering the EvidenceGraph as a
    /// DFS tree. Nodes that are already printed (convergence / diamond pattern)
    /// show a back-reference instead of repeating their subtree.
    pub fn print_terminal(&self) {
        let severity_label = match self.severity {
            Severity::Info    => "\x1b[32m[Info]\x1b[0m",
            Severity::Warning => "\x1b[33m[Warning]\x1b[0m",
            Severity::Error   => "\x1b[31m[Error]\x1b[0m",
        };

        println!("\n\x1b[1m{} {}\x1b[0m", self.subject, severity_label);
        println!("  Cause: {}", self.cause);

        if let Err(e) = self.graph.validate() {
            println!("  \x1b[31mGraph error: {}\x1b[0m", e);
            return;
        }

        if self.graph.nodes.is_empty() {
            if let Some(ref s) = self.suggestion {
                println!("  Suggestion: \x1b[36m{}\x1b[0m", s);
            }
            return;
        }

        println!("  Evidence Graph:");
        let mut printed: HashSet<String> = HashSet::new();

        for root in self.graph.roots() {
            self.render_node(root, "", true, &mut printed);
        }

        // Orphan nodes (disconnected from all roots)
        for node in &self.graph.nodes {
            if !printed.contains(&node.id) {
                let suffix = val_suffix(&node.value);
                println!("    {} [{}]{}", node.label, node.node_type, suffix);
            }
        }

        if let Some(ref s) = self.suggestion {
            println!("  Suggestion: \x1b[36m{}\x1b[0m", s);
        }
    }

    fn render_node(
        &self,
        node: &EvidenceNode,
        indent: &str,
        is_last: bool,
        printed: &mut HashSet<String>,
    ) {
        printed.insert(node.id.clone());

        let pfx = if indent.is_empty() { "  " } else { "  " };
        let suffix = val_suffix(&node.value);
        println!("{}{}\x1b[1m{}\x1b[0m [{}]{}", pfx, indent, node.label, node.node_type, suffix);

        let children = self.graph.children(&node.id);
        let n = children.len();

        for (i, (rel, child)) in children.into_iter().enumerate() {
            let child_last = i == n - 1;
            // Tree branch characters
            let branch  = if child_last { "└─" } else { "├─" };
            let vbar    = if child_last { "  " } else { "│ " };
            let rel_indent = format!("{}{}", indent, if is_last { "  " } else { "│ " });

            println!("  {}{} \x1b[35m↓ {}\x1b[0m", pfx, rel_indent, rel.relation);

            let child_indent = format!("{}{}", rel_indent, vbar);

            if printed.contains(&child.id) {
                // Convergence: this node was already fully rendered above. Show a back-ref.
                let suffix = val_suffix(&child.value);
                println!(
                    "  {}{}{} \x1b[1m{}\x1b[0m [{}]{} \x1b[90m(↖ see above)\x1b[0m",
                    pfx, rel_indent, branch, child.label, child.node_type, suffix
                );
            } else {
                println!("  {}{}{}", pfx, rel_indent, branch);
                self.render_node(child, &child_indent, child_last, printed);
            }
        }
    }
}

fn val_suffix(v: &str) -> String {
    if v.is_empty() { String::new() } else { format!(" ({})", v) }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str) -> EvidenceNode {
        EvidenceNode { id: id.to_string(), node_type: "T".to_string(), label: id.to_string(), value: "".to_string() }
    }
    fn rel(from: &str, to: &str) -> Relationship {
        Relationship { from: from.to_string(), relation: "->".to_string(), to: to.to_string() }
    }

    // ── Validation ────────────────────────────────────────────────────────────

    #[test]
    fn test_valid_linear_graph() {
        let g = EvidenceGraph::new(
            vec![node("a"), node("b"), node("c")],
            vec![rel("a", "b"), rel("b", "c")],
        );
        assert!(g.validate().is_ok());
    }

    #[test]
    fn test_valid_branching_graph() {
        let g = EvidenceGraph::new(
            vec![node("a"), node("b"), node("c")],
            vec![rel("a", "b"), rel("a", "c")],
        );
        assert!(g.validate().is_ok());
        assert_eq!(g.roots().len(), 1);
        assert_eq!(g.roots()[0].id, "a");
        assert_eq!(g.children("a").len(), 2);
    }

    #[test]
    fn test_duplicate_node_id() {
        let g = EvidenceGraph::new(vec![node("a"), node("a")], vec![]);
        assert!(matches!(g.validate(), Err(GraphValidationError::DuplicateNodeId(_))));
    }

    #[test]
    fn test_missing_node_reference_from() {
        let g = EvidenceGraph::new(vec![node("b")], vec![rel("ghost", "b")]);
        assert!(matches!(g.validate(), Err(GraphValidationError::MissingNodeReference(_))));
    }

    #[test]
    fn test_missing_node_reference_to() {
        let g = EvidenceGraph::new(vec![node("a")], vec![rel("a", "ghost")]);
        assert!(matches!(g.validate(), Err(GraphValidationError::MissingNodeReference(_))));
    }

    #[test]
    fn test_duplicate_relationship() {
        let g = EvidenceGraph::new(
            vec![node("a"), node("b")],
            vec![rel("a", "b"), rel("a", "b")],
        );
        assert!(matches!(g.validate(), Err(GraphValidationError::DuplicateRelationship(_, _, _))));
    }

    #[test]
    fn test_cycle_detection() {
        let g = EvidenceGraph::new(
            vec![node("a"), node("b"), node("c")],
            vec![rel("a", "b"), rel("b", "c"), rel("c", "a")],
        );
        assert!(matches!(g.validate(), Err(GraphValidationError::CycleDetected(_))));
    }

    #[test]
    fn test_roots_with_two_incoming() {
        // a → b, c → b  ⟹  a and c are roots; b is not
        let g = EvidenceGraph::new(
            vec![node("a"), node("b"), node("c")],
            vec![rel("a", "b"), rel("c", "b")],
        );
        let mut ids: Vec<&str> = g.roots().iter().map(|n| n.id.as_str()).collect();
        ids.sort();
        assert_eq!(ids, vec!["a", "c"]);
    }

    #[test]
    fn test_empty_graph_is_valid() {
        assert!(EvidenceGraph::new(vec![], vec![]).validate().is_ok());
    }

    // ── Convergence: two roots share the same leaf ────────────────────────────

    #[test]
    fn test_convergence_graph_is_valid() {
        // a → c, b → c  (diamond / convergence)
        let g = EvidenceGraph::new(
            vec![node("a"), node("b"), node("c")],
            vec![rel("a", "c"), rel("b", "c")],
        );
        assert!(g.validate().is_ok());
        // Both a and b are roots
        let mut roots: Vec<&str> = g.roots().iter().map(|n| n.id.as_str()).collect();
        roots.sort();
        assert_eq!(roots, vec!["a", "b"]);
        // c is still reachable as a child of both
        assert_eq!(g.children("a").len(), 1);
        assert_eq!(g.children("b").len(), 1);
    }

    // ── trace() ───────────────────────────────────────────────────────────────

    #[test]
    fn test_trace_direct_edge() {
        let g = EvidenceGraph::new(
            vec![node("a"), node("b")],
            vec![rel("a", "b")],
        );
        let path = g.trace("a", "b").expect("should find path");
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], ("a".into(), "->".into(), "b".into()));
    }

    #[test]
    fn test_trace_multi_hop() {
        let g = EvidenceGraph::new(
            vec![node("a"), node("b"), node("c"), node("d")],
            vec![rel("a", "b"), rel("b", "c"), rel("c", "d")],
        );
        let path = g.trace("a", "d").expect("should find path");
        assert_eq!(path.len(), 3);
        assert_eq!(path[2].2, "d");
    }

    #[test]
    fn test_trace_no_path() {
        let g = EvidenceGraph::new(
            vec![node("a"), node("b")],
            vec![],
        );
        assert!(g.trace("a", "b").is_none());
    }

    #[test]
    fn test_trace_same_node() {
        let g = EvidenceGraph::new(vec![node("a")], vec![]);
        assert_eq!(g.trace("a", "a"), Some(vec![]));
    }
}
