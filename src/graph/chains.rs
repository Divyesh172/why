use std::path::Path;

use crate::inspectors::executable::{
    query_version,
    installation::detect_installation_source,
    conflict::share_installation_root,
    projects::scan_projects,
};
use crate::inspectors::finding::{Finding, Severity};
use crate::platform;
use crate::resolver::path::find_all_in_path;
use super::SystemGraph;

// ─────────────────────────────────────────────────────────────────────────────
// Port → PID → Exe → Runtime → Project causal chain
// ─────────────────────────────────────────────────────────────────────────────

/// Builds the full cross-system causal chain for a TCP port number.
///
/// ```
/// Port 8080
///   └─ owned by
///        PID 18472 (node.exe)
///          └─ executable
///               node.exe [Binary]
///                 ├─ installed by
///                 │    fnm [PackageManager]
///                 └─ resolves to
///                      Node.js v24 [Runtime]
///                        └─ used by
///                             my-api [Project]
/// ```
pub fn build_port_chain(port: u16) -> Finding {
    let mut sg = SystemGraph::new();

    let port_id = format!("port:{}", port);
    sg.node(&port_id, "Port", &format!("Port {}", port), "");

    match platform::get_port_info(port) {
        Some(conn) => {
            // PID
            let pid_id = format!("pid:{}", conn.pid);
            sg.node(&pid_id, "Process", &conn.name, &format!("PID {}", conn.pid));
            sg.edge(&port_id, "owned by", &pid_id);

            // Try to get full process metadata
            if let Some(proc) = platform::get_process_info(&conn.pid) {

                // Command line (truncated)
                if !proc.cmd_line.is_empty() {
                    let cmd_id = format!("cmd:{}", conn.pid);
                    let cmd_display = if proc.cmd_line.len() > 80 {
                        format!("{}…", &proc.cmd_line[..80])
                    } else {
                        proc.cmd_line.clone()
                    };
                    sg.node(&cmd_id, "Command", "Command Line", &cmd_display);
                    sg.edge(&pid_id, "running as", &cmd_id);
                }

                // Executable binary
                if !proc.path.is_empty() {
                    let exe_name = Path::new(&proc.path)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| proc.name.clone());
                    let exe_id = format!("exe:{}", exe_name.to_lowercase());

                    sg.node(&exe_id, "Binary", &exe_name, &proc.path);
                    sg.edge(&pid_id, "executable", &exe_id);

                    // Installation / package manager
                    let install = detect_installation_source(Path::new(&proc.path));
                    let mgr_id = format!("mgr:{}", install.manager.to_lowercase().replace([' ', '(', ')'], "_"));
                    sg.node(&mgr_id, "PackageManager", &install.manager, "");
                    sg.edge(&mgr_id, "manages", &exe_id);

                    // Runtime version
                    if let Some(ver) = query_version(Path::new(&proc.path)) {
                        let runtime_id = format!("runtime:{}", exe_name.to_lowercase());
                        let display = exe_display_name(&exe_name);
                        sg.node(&runtime_id, "Runtime", &display, &ver);
                        sg.edge(&exe_id, "resolves to", &runtime_id);

                        // Projects using this runtime
                        let query_name = exe_name.trim_end_matches(".exe").to_lowercase();
                        for proj in scan_projects(&query_name).into_iter().take(3) {
                            let proj_name = proj.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let proj_id = format!("project:{}", proj_name);
                            sg.node(&proj_id, "Project", &proj_name, &proj.to_string_lossy());
                            sg.edge(&runtime_id, "used by", &proj_id);
                        }
                    }
                }

                // Parent process (if distinct from the PID itself)
                if !conn.parent.is_empty() && conn.parent != conn.pid {
                    let parent_id = format!("parent:{}", conn.parent);
                    sg.node(&parent_id, "Process", &format!("Parent ({})", conn.parent), "");
                    sg.edge(&pid_id, "spawned by", &parent_id);
                }
            }

            sg.into_finding(
                Severity::Info,
                format!("Port {}", port),
                format!("Port {} is occupied by {} (PID {}).", port, conn.name, conn.pid),
                None,
            )
        }
        None => sg.into_finding(
            Severity::Info,
            format!("Port {}", port),
            format!("Port {} is free — nothing is listening.", port),
            None,
        ),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Executable: PATH → Binary → Package Manager → Runtime → Projects
// ─────────────────────────────────────────────────────────────────────────────

/// Builds the full cross-system causal chain for a command name on PATH.
///
/// ```
/// System PATH [Environment]
///   ├─ resolves (entry #2)
///   │    node.exe [Binary]
///   │      ├─ installed by
///   │      │    fnm [PackageManager]
///   │      └─ resolves to
///   │           Node.js v24 [Runtime]
///   │             └─ used by
///   │                  my-api [Project]
///   └─ also found (entry #4)
///        node.exe (alt) [Binary]  ← different install
/// ```
pub fn build_executable_chain(query: &str) -> Finding {
    let (all_results, _) = find_all_in_path(query);

    if all_results.is_empty() {
        let mut sg = SystemGraph::new();
        sg.node("path", "Environment", "System PATH", "");
        let exe_id = format!("exe:{}", query);
        sg.node(&exe_id, "Binary", &format!("{}.exe", query), "Not Found");
        sg.edge("path", "searched for", &exe_id);
        return sg.into_finding(
            Severity::Error,
            query.to_string(),
            format!("'{}' was not found in any PATH entry.", query),
            Some(format!("Install {} or verify your PATH configuration.", query)),
        );
    }

    let mut sg = SystemGraph::new();
    sg.node("path", "Environment", "System PATH", "");

    // Active (first-resolved) installation
    let active = &all_results[0];
    let active_exe_name = Path::new(&active.resolved_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("{}.exe", query));

    let exe_id = format!("exe:{}", query);
    sg.node(&exe_id, "Binary", &active_exe_name, &active.resolved_path.to_string_lossy());
    sg.edge("path", "resolves", &exe_id);

    // Package manager
    let install = detect_installation_source(&active.resolved_path);
    let mgr_id = format!("mgr:{}", install.manager.to_lowercase().replace([' ', '(', ')'], "_"));
    sg.node(&mgr_id, "PackageManager", &install.manager,
        install.detail.as_deref().unwrap_or(""));
    sg.edge(&mgr_id, "manages", &exe_id);

    // Runtime version
    if let Some(ver) = query_version(&active.resolved_path) {
        let runtime_id = format!("runtime:{}", query);
        let display = exe_display_name(&active_exe_name);
        sg.node(&runtime_id, "Runtime", &display, &ver);
        sg.edge(&exe_id, "resolves to", &runtime_id);

        // Projects using this runtime
        for proj in scan_projects(query).into_iter().take(3) {
            let proj_name = proj.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let proj_id = format!("project:{}", proj_name);
            sg.node(&proj_id, "Project", &proj_name, &proj.to_string_lossy());
            sg.edge(&runtime_id, "used by", &proj_id);
        }
    }

    // Sibling-tool conflict check (node ↔ npm, python ↔ pip)
    let mut conflict_note: Option<String> = None;
    let sibling = match query {
        "node" => Some("npm"),
        "python" | "python3" => Some("pip"),
        _ => None,
    };
    if let Some(sib) = sibling {
        let (sib_results, _) = find_all_in_path(sib);
        if let Some(sib_res) = sib_results.first() {
            if !share_installation_root(&active.resolved_path, &sib_res.resolved_path) {
                let sib_id = format!("exe:{}", sib);
                sg.node(&sib_id, "Binary", &format!("{}.exe", sib), &sib_res.resolved_path.to_string_lossy());
                sg.edge("path", &format!("{} resolves separately", sib), &sib_id);
                conflict_note = Some(format!(
                    "{} and {} resolve to different installations — possible version mismatch.",
                    query, sib
                ));
            }
        }
    }

    // Shadow installations (entries 2..N on PATH)
    for (i, alt) in all_results.iter().enumerate().skip(1) {
        let alt_id = format!("exe:{}_alt{}", query, i);
        let alt_name = Path::new(&alt.resolved_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let alt_install = detect_installation_source(&alt.resolved_path);
        sg.node(
            &alt_id, "Binary",
            &format!("{}.exe [shadowed, entry #{}]", alt_name, i + 1),
            &alt.resolved_path.to_string_lossy(),
        );
        sg.edge("path", &format!("also found (entry #{}, via {})", i + 1, alt_install.manager), &alt_id);
    }

    let severity = if all_results.len() > 1 || conflict_note.is_some() {
        Severity::Warning
    } else {
        Severity::Info
    };

    let cause = if all_results.len() > 1 {
        format!(
            "{} installations of '{}' found on PATH. Active: {} via {}.",
            all_results.len(), query,
            query_version(&active.resolved_path).unwrap_or_else(|| "unknown".to_string()),
            install.manager,
        )
    } else {
        format!(
            "'{}' resolves to {} via {}.",
            query,
            query_version(&active.resolved_path).unwrap_or_else(|| "unknown version".to_string()),
            install.manager,
        )
    };

    sg.into_finding(severity, query.to_string(), cause, conflict_note)
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn exe_display_name(exe: &str) -> String {
    match exe.trim_end_matches(".exe").to_lowercase().as_str() {
        "node"             => "Node.js".into(),
        "python" | "python3" => "Python".into(),
        "java"             => "Java".into(),
        "ruby"             => "Ruby".into(),
        "go"               => "Go".into(),
        "cargo" | "rustc"  => "Rust".into(),
        "docker"           => "Docker".into(),
        "git"              => "Git".into(),
        other              => other.to_string(),
    }
}
