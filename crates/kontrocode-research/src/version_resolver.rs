use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DepInfo {
    pub name: String,
    pub version: String,
    pub source: String,
    pub is_direct: bool,
}

#[derive(Debug, Clone)]
pub struct ConflictReport {
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
}

pub struct VersionResolver;

impl VersionResolver {
    pub fn check_conflicts(deps: &[DepInfo]) -> ConflictReport {
        let mut conflicts = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();
        let mut seen: HashMap<String, Vec<&DepInfo>> = HashMap::new();

        for dep in deps {
            seen.entry(dep.name.clone()).or_default().push(dep);
        }

        for (name, versions) in &seen {
            if versions.len() > 1 {
                let vers: Vec<&str> = versions.iter().map(|d| d.version.as_str()).collect();
                conflicts.push(format!("{name}: multiple versions — {}", vers.join(", ")));
            }
        }

        for dep in deps {
            if dep.version.starts_with('0') {
                warnings.push(format!("{name}@{ver} is pre-1.0, may have breaking changes", name = dep.name, ver = dep.version));
            }
            if dep.version.contains("alpha") || dep.version.contains("beta") || dep.version.contains("rc") {
                warnings.push(format!("{name}@{ver} is pre-release", name = dep.name, ver = dep.version));
            }
        }

        if conflicts.is_empty() && warnings.is_empty() {
            suggestions.push("No version conflicts detected".into());
        }

        ConflictReport { conflicts, warnings, suggestions }
    }

    pub fn parse_cargo_toml(path: &Path) -> Option<Vec<DepInfo>> {
        let content = std::fs::read_to_string(path).ok()?;
        let mut deps = Vec::new();
        let mut in_deps = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "[dependencies]" { in_deps = true; continue; }
            if trimmed.starts_with('[') && trimmed != "[dependencies]" { in_deps = false; continue; }
            if in_deps && !trimmed.is_empty() && !trimmed.starts_with('#') {
                if let Some((name, version)) = trimmed.split_once('=') {
                    let ver = version.trim().trim_matches('"').trim_matches('\'');
                    deps.push(DepInfo {
                        name: name.trim().to_string(),
                        version: ver.to_string(),
                        source: "Cargo.toml".into(),
                        is_direct: true,
                    });
                }
            }
        }
        Some(deps)
    }

    pub fn parse_package_json(path: &Path) -> Option<Vec<DepInfo>> {
        let content = std::fs::read_to_string(path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;
        let mut deps = Vec::new();
        if let Some(obj) = json["dependencies"].as_object() {
            for (name, ver) in obj {
                let v = ver.as_str().unwrap_or("latest");
                deps.push(DepInfo {
                    name: name.clone(),
                    version: v.to_string(),
                    source: "package.json".into(),
                    is_direct: true,
                });
            }
        }
        if let Some(obj) = json["devDependencies"].as_object() {
            for (name, ver) in obj {
                let v = ver.as_str().unwrap_or("latest");
                deps.push(DepInfo {
                    name: name.clone(),
                    version: v.to_string(),
                    source: "package.json (dev)".into(),
                    is_direct: false,
                });
            }
        }
        Some(deps)
    }

    pub fn parse_pubspec(path: &Path) -> Option<Vec<DepInfo>> {
        let content = std::fs::read_to_string(path).ok()?;
        let mut deps = Vec::new();
        let mut in_deps = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "dependencies:" { in_deps = true; continue; }
            if trimmed == "dev_dependencies:" { in_deps = true; continue; }
            if !trimmed.starts_with(' ') && !trimmed.is_empty() && trimmed.contains(':') {
                in_deps = false;
                continue;
            }
            if in_deps && !trimmed.is_empty() && !trimmed.starts_with('#') {
                let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
                if parts.len() >= 2 {
                    deps.push(DepInfo {
                        name: parts[0].trim().to_string(),
                        version: parts[1].trim().to_string(),
                        source: "pubspec.yaml".into(),
                        is_direct: true,
                    });
                }
            }
        }
        Some(deps)
    }
}
