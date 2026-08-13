use std::path::Path;

#[derive(Debug)]
pub struct InstallationInfo {
    pub manager: String,
    pub detail: Option<String>,
}

/// Inspects the file path to determine the installation source.
pub fn detect_installation_source(path: &Path) -> InstallationInfo {
    let path_str = path.to_string_lossy().to_lowercase();
    
    if path_str.contains("scoop\\apps") {
        let parts: Vec<String> = path.components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();
            
        if let Some(apps_idx) = parts.iter().position(|x| x.to_lowercase() == "apps") {
            if apps_idx + 1 < parts.len() {
                let app_name = &parts[apps_idx + 1];
                return InstallationInfo {
                    manager: "Scoop".to_string(),
                    detail: Some(format!("App directory: {}", app_name)),
                };
            }
        }
        return InstallationInfo {
            manager: "Scoop".to_string(),
            detail: None,
        };
    }
    
    if path_str.contains("fnm_multishells") || path_str.contains("fnm\\") {
        return InstallationInfo {
            manager: "fnm (Fast Node Manager)".to_string(),
            detail: Some("Managed Node.js environment".to_string()),
        };
    }

    if path_str.contains(".cargo\\bin") {
        return InstallationInfo {
            manager: "Cargo (Rust)".to_string(),
            detail: Some("Installed via 'cargo install'".to_string()),
        };
    }

    if path_str.contains("chocolatey\\bin") || path_str.contains("programdata\\chocolatey") {
        return InstallationInfo {
            manager: "Chocolatey".to_string(),
            detail: None,
        };
    }

    if path_str.contains("winget\\packages") || path_str.contains("microsoft\\winget") {
        return InstallationInfo {
            manager: "WinGet".to_string(),
            detail: None,
        };
    }

    InstallationInfo {
        manager: "System / Manual Installation".to_string(),
        detail: None,
    }
}
