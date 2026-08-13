use serde::Serialize;
use std::collections::HashSet;

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
            GraphValidationError::DuplicateNodeId(id) => write!(f, "Duplicate node ID: {}", id),
            GraphValidationError::MissingNodeReference(id) => write!(f, "Relationship references missing node ID: {}", id),
            GraphValidationError::DuplicateRelationship(from, rel, to) => write!(f, "Duplicate relationship: {} -[{}]-> {}", from, rel, to),
            GraphValidationError::CycleDetected(id) => write!(f, "Cycle detected starting at node ID: {}", id),
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
pub struct EvidenceGraph {
    pub nodes: Vec<EvidenceNode>,
    pub relationships: Vec<Relationship>,
}

impl EvidenceGraph {
    pub fn new(nodes: Vec<EvidenceNode>, relationships: Vec<Relationship>) -> Self {
        Self { nodes, relationships }
    }

    /// Validates graph constraints: no cycle loops, no duplicate nodes or relationships, and all references must exist.
    pub fn validate(&self) -> Result<(), GraphValidationError> {
        let mut node_ids = HashSet::new();
        for node in &self.nodes {
            if !node_ids.insert(&node.id) {
                return Err(GraphValidationError::DuplicateNodeId(node.id.clone()));
            }
        }

        let mut rels_seen = HashSet::new();
        for rel in &self.relationships {
            if !node_ids.contains(&rel.from) {
                return Err(GraphValidationError::MissingNodeReference(rel.from.clone()));
            }
            if !node_ids.contains(&rel.to) {
                return Err(GraphValidationError::MissingNodeReference(rel.to.clone()));
            }
            let key = (rel.from.clone(), rel.relation.clone(), rel.to.clone());
            if !rels_seen.insert(key) {
                return Err(GraphValidationError::DuplicateRelationship(rel.from.clone(), rel.relation.clone(), rel.to.clone()));
            }
        }

        // Cycle detection using DFS traversal recursion stacks
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        
        for node in &self.nodes {
            if self.detect_cycle(&node.id, &mut visited, &mut rec_stack) {
                return Err(GraphValidationError::CycleDetected(node.id.clone()));
            }
        }

        Ok(())
    }

    fn detect_cycle(&self, node_id: &str, visited: &mut HashSet<String>, rec_stack: &mut HashSet<String>) -> bool {
        if rec_stack.contains(node_id) {
            return true;
        }
        if visited.contains(node_id) {
            return false;
        }

        visited.insert(node_id.to_string());
        rec_stack.insert(node_id.to_string());

        for rel in &self.relationships {
            if rel.from == node_id {
                if self.detect_cycle(&rel.to, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(node_id);
        false
    }

    /// Finds all root source nodes (nodes with no incoming relationships).
    pub fn roots(&self) -> Vec<&EvidenceNode> {
        self.nodes.iter()
            .filter(|node| !self.relationships.iter().any(|rel| rel.to == node.id))
            .collect()
    }

    /// Returns child nodes matching a parent ID with their respective relationships.
    pub fn children(&self, parent_id: &str) -> Vec<(&Relationship, &EvidenceNode)> {
        let mut result = Vec::new();
        for rel in &self.relationships {
            if rel.from == parent_id {
                if let Some(target) = self.nodes.iter().find(|n| n.id == rel.to) {
                    result.push((rel, target));
                }
            }
        }
        result
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct Finding {
    pub severity: Severity,
    pub subject: String,
    pub cause: String,
    pub graph: EvidenceGraph,
    pub suggestion: Option<String>,
}

impl Finding {
    /// Recursively traces and outputs the Evidence Graph with branch connector lines.
    pub fn print_terminal(&self) {
        let severity_color = match self.severity {
            Severity::Info => "\x1b[32m[Info]\x1b[0m",
            Severity::Warning => "\x1b[33m[Warning]\x1b[0m",
            Severity::Error => "\x1b[31m[Error]\x1b[0m",
        };

        println!("\n\x1b[1m{} {}\x1b[0m", self.subject, severity_color);
        println!("  Cause: {}", self.cause);
        
        if let Err(e) = self.graph.validate() {
            println!("  \x1b[31mError: Invalid Evidence Graph: {}\x1b[0m", e);
            return;
        }

        println!("  Evidence Graph:");
        let mut printed_nodes = HashSet::new();

        let roots = self.graph.roots();
        for root in roots {
            self.print_node_tree(root, "", true, &mut printed_nodes);
        }

        // Print disconnected nodes
        for node in &self.graph.nodes {
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

    fn print_node_tree(&self, node: &EvidenceNode, indent: &str, is_last: bool, printed: &mut HashSet<String>) {
        printed.insert(node.id.clone());

        let val_suffix = if node.value.is_empty() {
            "".to_string()
        } else {
            format!(" ({})", node.value)
        };

        println!("    {}{}{} [{}]{}", indent, if indent.is_empty() { "" } else { " " }, node.label, node.node_type, val_suffix);

        let children = self.graph.children(&node.id);
        let children_len = children.len();

        for (i, (rel, child)) in children.into_iter().enumerate() {
            let is_child_last = i == children_len - 1;
            let next_indent = format!("{}{}", indent, if is_last { "    " } else { "│   " });
            println!("    {} \x1b[35m↓ {}\x1b[0m", next_indent, rel.relation);
            self.print_node_tree(child, &next_indent, is_child_last, printed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str) -> EvidenceNode {
        EvidenceNode { id: id.to_string(), node_type: "Test".to_string(), label: id.to_string(), value: "".to_string() }
    }

    fn rel(from: &str, to: &str) -> Relationship {
        Relationship { from: from.to_string(), relation: "->".to_string(), to: to.to_string() }
    }

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
        // a -> b, a -> c  (a has two children)
        let g = EvidenceGraph::new(
            vec![node("a"), node("b"), node("c")],
            vec![rel("a", "b"), rel("a", "c")],
        );
        assert!(g.validate().is_ok());
        let roots = g.roots();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].id, "a");
        let children = g.children("a");
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_duplicate_node_id() {
        let g = EvidenceGraph::new(
            vec![node("a"), node("a")],
            vec![],
        );
        assert!(matches!(g.validate(), Err(GraphValidationError::DuplicateNodeId(_))));
    }

    #[test]
    fn test_missing_node_reference_from() {
        let g = EvidenceGraph::new(
            vec![node("b")],
            vec![rel("ghost", "b")],
        );
        assert!(matches!(g.validate(), Err(GraphValidationError::MissingNodeReference(_))));
    }

    #[test]
    fn test_missing_node_reference_to() {
        let g = EvidenceGraph::new(
            vec![node("a")],
            vec![rel("a", "ghost")],
        );
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
        // a -> b -> c -> a (cycle)
        let g = EvidenceGraph::new(
            vec![node("a"), node("b"), node("c")],
            vec![rel("a", "b"), rel("b", "c"), rel("c", "a")],
        );
        assert!(matches!(g.validate(), Err(GraphValidationError::CycleDetected(_))));
    }

    #[test]
    fn test_roots_returns_correct_nodes() {
        // a -> b, c -> b: b has two incoming, a and c are roots
        let g = EvidenceGraph::new(
            vec![node("a"), node("b"), node("c")],
            vec![rel("a", "b"), rel("c", "b")],
        );
        let mut root_ids: Vec<&str> = g.roots().iter().map(|n| n.id.as_str()).collect();
        root_ids.sort();
        assert_eq!(root_ids, vec!["a", "c"]);
    }

    #[test]
    fn test_empty_graph_is_valid() {
        let g = EvidenceGraph::new(vec![], vec![]);
        assert!(g.validate().is_ok());
    }
}
