//! Module-boundary rules (CLAUDE.md §4.2.7) and vendor dependency rules (ADR 0001).

use anyhow::{Context, bail};
use cargo_metadata::{MetadataCommand, Package};

const FORBIDDEN_IN_CONTRACTS: &[&str] = &["sqlx", "axum", "tokio", "redis", "reqwest"];
const FORBIDDEN_IN_CORE: &[&str] = &[];

/// A package reduced to what the vendor rules need; `manifest_path` is relative to the workspace root.
#[derive(Debug, Clone)]
pub(crate) struct PackageInfo {
    pub(crate) name: String,
    pub(crate) manifest_path: String,
    pub(crate) deps: Vec<String>,
}

impl PackageInfo {
    fn is_vendor(&self) -> bool {
        self.manifest_path.starts_with("vendor/")
    }

    fn is_adapter(&self) -> bool {
        self.manifest_path
            .strip_prefix("modules/")
            .and_then(|rest| rest.split('/').nth(1))
            == Some("engine")
    }
}

/// Returns one message per violation of the vendor rules:
/// only `modules/<m>/engine` may depend on vendor crates, and vendor crates must not
/// depend on any non-vendor package that is part of the set.
pub(crate) fn vendor_violations(pkgs: &[PackageInfo]) -> Vec<String> {
    let mut out = Vec::new();
    for pkg in pkgs {
        for dep in &pkg.deps {
            let Some(target) = pkgs.iter().find(|p| &p.name == dep) else {
                continue;
            };
            if target.is_vendor() && !pkg.is_vendor() && !pkg.is_adapter() {
                out.push(format!(
                    "{}: depends on vendor crate `{dep}` (only modules/*/engine may)",
                    pkg.name
                ));
            }
            if pkg.is_vendor() && !target.is_vendor() {
                out.push(format!("{}: depends on workspace crate `{dep}`", pkg.name));
            }
        }
    }
    out
}

fn module_of(pkg: &Package) -> Option<(String, bool)> {
    let path = pkg.manifest_path.as_str();
    let rest = path.split("/modules/").nth(1)?;
    let mut parts = rest.split('/');
    let module = parts.next()?.to_owned();
    let is_contracts = parts.next()? == "contracts";
    Some((module, is_contracts))
}

/// Checks module boundaries and vendor rules over the whole dependency graph.
pub(crate) fn check() -> anyhow::Result<()> {
    let meta = MetadataCommand::new().exec().context("cargo metadata")?;
    let members: Vec<&Package> = meta
        .packages
        .iter()
        .filter(|p| meta.workspace_members.contains(&p.id))
        .collect();
    let mut violations = Vec::new();

    for pkg in &members {
        let Some((module, is_contracts)) = module_of(pkg) else {
            continue;
        };
        let forbidden = if is_contracts {
            FORBIDDEN_IN_CONTRACTS
        } else {
            FORBIDDEN_IN_CORE
        };
        for dep in &pkg.dependencies {
            if forbidden.contains(&dep.name.as_str()) {
                violations.push(format!("{}: forbidden dependency `{}`", pkg.name, dep.name));
            }
            // Only another module's contracts crate may be depended upon.
            if let Some(other) = members.iter().find(|m| m.name == dep.name)
                && let Some((other_module, other_is_contracts)) = module_of(other)
                && other_module != module
                && !other_is_contracts
            {
                violations.push(format!(
                    "{}: depends on core of module `{other_module}`",
                    pkg.name
                ));
            }
        }
    }

    let infos: Vec<PackageInfo> = meta
        .packages
        .iter()
        .filter(|p| {
            meta.workspace_members.contains(&p.id) || p.manifest_path.as_str().contains("/vendor/")
        })
        .map(|p| PackageInfo {
            name: p.name.to_string(),
            manifest_path: p.manifest_path.to_string(),
            deps: p.dependencies.iter().map(|d| d.name.clone()).collect(),
        })
        .filter(|i| i.manifest_path.starts_with("vendor/") || !i.manifest_path.starts_with('/'))
        .collect();
    violations.extend(vendor_violations(&infos));

    if violations.is_empty() {
        Ok(())
    } else {
        bail!("architecture violations:\n{}", violations.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(name: &str, path: &str, deps: &[&str]) -> PackageInfo {
        PackageInfo {
            name: name.to_owned(),
            manifest_path: path.to_owned(),
            deps: deps.iter().map(|d| (*d).to_owned()).collect(),
        }
    }

    fn vendor() -> PackageInfo {
        pkg(
            "gridcraft-formula",
            "vendor/gridcraft/crates/formula/Cargo.toml",
            &[],
        )
    }

    #[test]
    fn adapter_may_use_vendor() {
        let adapter = pkg(
            "spreadsheets-engine",
            "modules/spreadsheets/engine/Cargo.toml",
            &["gridcraft-formula"],
        );
        assert!(vendor_violations(&[adapter, vendor()]).is_empty());
    }

    #[test]
    fn core_using_vendor_is_violation() {
        let core = pkg(
            "spreadsheets",
            "modules/spreadsheets/core/Cargo.toml",
            &["gridcraft-formula"],
        );
        assert_eq!(
            vendor_violations(&[core, vendor()]),
            [
                "spreadsheets: depends on vendor crate `gridcraft-formula` (only modules/*/engine may)"
            ]
        );
    }

    #[test]
    fn vendor_using_workspace_crate_is_violation() {
        let ours = pkg("platform", "crates/platform/Cargo.toml", &[]);
        let mut v = vendor();
        v.deps.push("platform".to_owned());
        assert_eq!(
            vendor_violations(&[v, ours]),
            ["gridcraft-formula: depends on workspace crate `platform`"]
        );
    }

    #[test]
    fn checkout_below_a_vendor_directory_does_not_make_everything_vendor() {
        // Paths are workspace-relative, so a parent directory named `vendor` is irrelevant.
        let core = pkg(
            "spreadsheets",
            "modules/spreadsheets/core/Cargo.toml",
            &["platform"],
        );
        let ours = pkg("platform", "crates/platform/Cargo.toml", &[]);
        assert!(vendor_violations(&[core, ours]).is_empty());
        assert!(!PackageInfo::is_vendor(&pkg(
            "x",
            "/home/vendor/r/crates/x/Cargo.toml",
            &[]
        )));
    }

    #[test]
    fn non_vendor_without_vendor_deps_is_clean() {
        let a = pkg("platform", "crates/platform/Cargo.toml", &["serde"]);
        assert!(vendor_violations(&[a, vendor()]).is_empty());
    }
}
