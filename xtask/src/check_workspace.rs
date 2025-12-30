use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use cargo_toml::{Dependency, Inheritable, Manifest};
use clap::Args;
use std::fs;

#[derive(Args)]
pub struct CheckWorkspaceArgs {}

/// Aggregated workspace manifests/config.
///
/// Parse inputs once, then apply independent rule functions.
struct WorkspaceManifest {
    root: PathBuf,
    root_manifest_path: PathBuf,
    root_manifest: Manifest,
    workspace_deps: BTreeMap<String, Dependency>,
    members: Vec<MemberManifest>,
    release_plz: Option<ReleasePlzConfig>,
}

struct MemberManifest {
    manifest_path: PathBuf,
    package_name: String,
    manifest: Manifest,
}

struct ReleasePlzConfig {
    path: PathBuf,
    by_name_version_group: BTreeMap<String, Option<String>>,
}

pub fn run(_args: CheckWorkspaceArgs) -> Result<()> {
    let root = crate::findup::workspace_root().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let ws = load_workspace(root)?;

    let mut errors = Vec::new();
    errors.extend(rule_root_workspace_package_version_present(&ws));
    errors.extend(rule_zeroos_family_versions_inherited(&ws));
    errors.extend(rule_workspace_deps_are_inherited(&ws));
    errors.extend(rule_no_local_crates_io_versions(&ws));
    errors.extend(rule_release_plz_zeroos_version_group_complete(&ws));

    finish(errors)
}

fn load_workspace(root: PathBuf) -> Result<WorkspaceManifest> {
    let root_manifest_path = root.join("Cargo.toml");
    let root_manifest =
        Manifest::from_path(&root_manifest_path).context("Failed to parse root Cargo.toml")?;

    let workspace_members = root_manifest
        .workspace
        .as_ref()
        .map(|w| w.members.clone())
        .unwrap_or_default();

    let workspace_deps = root_manifest
        .workspace
        .as_ref()
        .map(|w| w.dependencies.clone())
        .unwrap_or_default();

    let mut members: Vec<MemberManifest> = Vec::new();
    for member in workspace_members {
        let member_manifest_path = root.join(member).join("Cargo.toml");
        if !member_manifest_path.exists() {
            continue;
        }
        if let Some(m) = load_member_manifest(&member_manifest_path)? {
            members.push(m);
        }
    }

    let release_plz = load_release_plz(&root)?;

    Ok(WorkspaceManifest {
        root,
        root_manifest_path,
        root_manifest,
        workspace_deps,
        members,
        release_plz,
    })
}

fn load_member_manifest(path: &Path) -> Result<Option<MemberManifest>> {
    // IMPORTANT:
    // `cargo_toml::Manifest::from_path` calls `complete_from_path`, which resolves workspace
    // inheritance (e.g. `version.workspace = true`) into concrete values.
    //
    // For this xtask we want to enforce what was *literally written* in the member manifest,
    // so we parse without completion.
    let raw = fs::read_to_string(path)?;
    let manifest =
        Manifest::from_str(&raw).with_context(|| format!("Failed to parse {}", path.display()))?;

    let Some(package) = manifest.package.as_ref() else {
        return Ok(None);
    };

    Ok(Some(MemberManifest {
        manifest_path: path.to_path_buf(),
        package_name: package.name.clone(),
        manifest,
    }))
}

fn load_release_plz(root: &Path) -> Result<Option<ReleasePlzConfig>> {
    let path = root.join("release-plz.toml");
    if !path.exists() {
        return Ok(None);
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let doc: toml::Value =
        toml::from_str(&raw).with_context(|| format!("Failed to parse {}", path.display()))?;

    let mut by_name_version_group: BTreeMap<String, Option<String>> = BTreeMap::new();
    if let Some(pkgs) = doc.get("package").and_then(|v| v.as_array()) {
        for p in pkgs {
            let Some(tbl) = p.as_table() else { continue };
            let Some(name) = tbl.get("name").and_then(|v| v.as_str()) else {
                continue;
            };
            let vg = tbl
                .get("version_group")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Duplicate detection is a “rule”, but we can conveniently record it here by storing
            // only one and letting the rule compare.
            by_name_version_group.insert(name.to_string(), vg);
        }
    }

    Ok(Some(ReleasePlzConfig {
        path,
        by_name_version_group,
    }))
}

fn rule_root_workspace_package_version_present(ws: &WorkspaceManifest) -> Vec<String> {
    // If we enforce member crates to inherit `version.workspace = true`, the root workspace
    // must define `[workspace.package].version`.
    let ws_pkg_version = ws
        .root_manifest
        .workspace
        .as_ref()
        .and_then(|w| w.package.as_ref())
        .and_then(|p| p.version.as_deref());

    if ws_pkg_version.is_some() {
        Vec::new()
    } else {
        vec![format!(
            "[workspace] ({}) must define [workspace.package].version for member crates to inherit `version.workspace = true`",
            ws.root_manifest_path.display()
        )]
    }
}

fn rule_zeroos_family_versions_inherited(ws: &WorkspaceManifest) -> Vec<String> {
    let mut errors = Vec::new();

    for m in &ws.members {
        let Some(pkg) = m.manifest.package.as_ref() else {
            continue;
        };
        let is_zeroos_family = pkg.name == "zeroos" || pkg.name.starts_with("zeroos-");
        if !is_zeroos_family {
            continue;
        }

        match &pkg.version {
            Inheritable::Inherited => {}
            _ => errors.push(format!(
                "[{}] ({}) version must be {{ workspace = true }}",
                m.package_name,
                m.manifest_path.display()
            )),
        }
    }

    errors
}

fn rule_workspace_deps_are_inherited(ws: &WorkspaceManifest) -> Vec<String> {
    let mut errors = Vec::new();

    for m in &ws.members {
        let Some(pkg) = m.manifest.package.as_ref() else {
            continue;
        };

        // If a dependency is defined in the workspace root, it MUST be inherited.
        check_dep_section_requires_inheritance(
            pkg.name.as_str(),
            &m.manifest_path,
            &ws.workspace_deps,
            &m.manifest.dependencies,
            "dependencies",
            &mut errors,
        );
        check_dep_section_requires_inheritance(
            pkg.name.as_str(),
            &m.manifest_path,
            &ws.workspace_deps,
            &m.manifest.dev_dependencies,
            "dev-dependencies",
            &mut errors,
        );
        check_dep_section_requires_inheritance(
            pkg.name.as_str(),
            &m.manifest_path,
            &ws.workspace_deps,
            &m.manifest.build_dependencies,
            "build-dependencies",
            &mut errors,
        );
    }

    errors
}

fn rule_no_local_crates_io_versions(ws: &WorkspaceManifest) -> Vec<String> {
    // Enforce that crates do not declare crates.io dependency versions locally.
    // Instead, they must centralize dependency definitions in the root `[workspace.dependencies]`
    // and reference them via `{ workspace = true }`.
    //
    // If you need an exception (e.g., truly crate-local tooling deps), add it here.
    const ALLOWED_NON_WORKSPACE_DEPS: &[&str] = &[];

    let mut errors = Vec::new();

    for m in &ws.members {
        let Some(pkg) = m.manifest.package.as_ref() else {
            continue;
        };

        check_dep_section_no_local_crates_io_versions(
            pkg.name.as_str(),
            &m.manifest_path,
            &ws.workspace_deps,
            &m.manifest.dependencies,
            "dependencies",
            ALLOWED_NON_WORKSPACE_DEPS,
            &mut errors,
        );
        check_dep_section_no_local_crates_io_versions(
            pkg.name.as_str(),
            &m.manifest_path,
            &ws.workspace_deps,
            &m.manifest.dev_dependencies,
            "dev-dependencies",
            ALLOWED_NON_WORKSPACE_DEPS,
            &mut errors,
        );
        check_dep_section_no_local_crates_io_versions(
            pkg.name.as_str(),
            &m.manifest_path,
            &ws.workspace_deps,
            &m.manifest.build_dependencies,
            "build-dependencies",
            ALLOWED_NON_WORKSPACE_DEPS,
            &mut errors,
        );
    }

    errors
}

fn rule_release_plz_zeroos_version_group_complete(ws: &WorkspaceManifest) -> Vec<String> {
    let Some(release_plz) = ws.release_plz.as_ref() else {
        return vec![format!(
            "[release-plz] ({}) missing `release-plz.toml`",
            ws.root.join("release-plz.toml").display()
        )];
    };

    // Collect all `zeroos` / `zeroos-*` member package names.
    let mut required: BTreeSet<&str> = BTreeSet::new();
    for m in &ws.members {
        if m.package_name == "zeroos" || m.package_name.starts_with("zeroos-") {
            required.insert(m.package_name.as_str());
        }
    }

    let mut errors = Vec::new();

    for pkg in required {
        match release_plz.by_name_version_group.get(pkg) {
            None => errors.push(format!(
                "[release-plz] ({}) missing [[package]] for '{}' (expected version_group = \"zeroos\")",
                release_plz.path.display(),
                pkg
            )),
            Some(None) => errors.push(format!(
                "[release-plz] ({}) package '{}' must set version_group = \"zeroos\"",
                release_plz.path.display(),
                pkg
            )),
            Some(Some(vg)) if vg != "zeroos" => errors.push(format!(
                "[release-plz] ({}) package '{}' has version_group = {:?}, expected \"zeroos\"",
                release_plz.path.display(),
                pkg,
                vg
            )),
            Some(Some(_)) => {}
        }
    }

    errors
}

fn check_dep_section_requires_inheritance(
    package_name: &str,
    manifest_path: &Path,
    workspace_deps: &BTreeMap<String, Dependency>,
    deps: &BTreeMap<String, Dependency>,
    section: &str,
    errors: &mut Vec<String>,
) {
    for (dep_name, dep_val) in deps {
        if !workspace_deps.contains_key(dep_name) {
            continue;
        }
        let is_workspace_ref = matches!(dep_val, Dependency::Inherited(_));
        if !is_workspace_ref {
            errors.push(format!(
                "[{}] ({}) dependency '{}' in '{}' must use {{ workspace = true }} because it is defined in the workspace root",
                package_name,
                manifest_path.display(),
                dep_name,
                section
            ));
        }
    }
}

fn check_dep_section_no_local_crates_io_versions(
    package_name: &str,
    manifest_path: &Path,
    workspace_deps: &BTreeMap<String, Dependency>,
    deps: &BTreeMap<String, Dependency>,
    section: &str,
    allowed_non_workspace_deps: &[&str],
    errors: &mut Vec<String>,
) {
    for (dep_name, dep_val) in deps {
        if allowed_non_workspace_deps.contains(&dep_name.as_str()) {
            continue;
        }

        // If it's in workspace deps, a separate rule checks that it is inherited.
        if workspace_deps.contains_key(dep_name) {
            continue;
        }

        match dep_val {
            Dependency::Inherited(_) => {
                errors.push(format!(
                    "[{}] ({}) dependency '{}' in '{}' uses {{ workspace = true }} but '{}' is not defined in the root [workspace.dependencies]",
                    package_name,
                    manifest_path.display(),
                    dep_name,
                    section,
                    dep_name
                ));
            }
            Dependency::Simple(_req) => {
                errors.push(format!(
                    "[{}] ({}) dependency '{}' in '{}' must be defined in root [workspace.dependencies] and referenced with {{ workspace = true }} (local version strings are not allowed)",
                    package_name,
                    manifest_path.display(),
                    dep_name,
                    section
                ));
            }
            Dependency::Detailed(d) => {
                let is_crates_io_like = d.path.is_none()
                    && d.git.is_none()
                    && d.registry.is_none()
                    && d.registry_index.is_none()
                    && d.tag.is_none()
                    && d.branch.is_none()
                    && d.rev.is_none();

                if is_crates_io_like && d.version.is_some() {
                    errors.push(format!(
                        "[{}] ({}) dependency '{}' in '{}' must be defined in root [workspace.dependencies] and referenced with {{ workspace = true }} (local version tables are not allowed)",
                        package_name,
                        manifest_path.display(),
                        dep_name,
                        section
                    ));
                }
            }
        }
    }
}

fn finish(errors: Vec<String>) -> Result<()> {
    if errors.is_empty() {
        println!("All zeroos crates checked successfully!");
        return Ok(());
    }

    eprintln!("Found {} workspace consistency errors:", errors.len());
    for err in errors {
        eprintln!("  - {}", err);
    }
    bail!("Workspace consistency check failed");
}
