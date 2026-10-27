//! Provenance checks for `vendor/` (ADR 0001).

use std::path::Path;

const REQUIRED_FILES: &[&str] = &["VENDOR.toml", "LICENSE-MIT", "LICENSE-APACHE", "NOTICE"];
const FORBIDDEN_PATHS: &[&str] = &["crates/ui-egui", "apps", "docs/brand"];

#[derive(serde::Deserialize)]
struct VendorFile {
    sha: String,
    crates: Vec<String>,
}

/// Returns one message per provenance violation under `<root>/vendor`.
pub(crate) fn vendor_check(root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(root.join("vendor")) else {
        return Vec::new();
    };
    let mut repos: Vec<_> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .collect();
    repos.sort_by_key(std::fs::DirEntry::file_name);
    let notice = std::fs::read_to_string(root.join("NOTICE")).unwrap_or_default();
    repos
        .iter()
        .flat_map(|e| check_repo(&e.file_name().to_string_lossy(), &e.path(), &notice))
        .collect()
}

fn check_repo(repo: &str, dir: &Path, notice: &str) -> Vec<String> {
    let mut out = Vec::new();
    for file in REQUIRED_FILES {
        if !dir.join(file).is_file() {
            out.push(format!("{repo}: missing {file}"));
        }
    }
    for path in FORBIDDEN_PATHS {
        if dir.join(path).exists() {
            out.push(format!("{repo}: forbidden path `{path}` present"));
        }
    }
    let Ok(raw) = std::fs::read_to_string(dir.join("VENDOR.toml")) else {
        return out;
    };
    let manifest: VendorFile = match toml::from_str(&raw) {
        Ok(m) => m,
        Err(err) => {
            out.push(format!("{repo}: invalid VENDOR.toml: {err}"));
            return out;
        }
    };
    if manifest.sha.len() != 40 || !manifest.sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        out.push(format!(
            "{repo}: bad sha `{}` (need 40 hex chars)",
            manifest.sha
        ));
    }
    if !notice.contains(&format!("vendor/{repo}")) || !notice.contains(&manifest.sha) {
        out.push(format!(
            "{repo}: root NOTICE does not list vendor/{repo} at sha {}",
            manifest.sha
        ));
    }
    let mut on_disk: Vec<String> = std::fs::read_dir(dir.join("crates"))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    on_disk.sort();
    for name in &on_disk {
        if !manifest.crates.contains(name) {
            out.push(format!(
                "{repo}: crate `{name}` present but not listed in VENDOR.toml"
            ));
        }
    }
    for name in &manifest.crates {
        if !on_disk.contains(name) {
            out.push(format!(
                "{repo}: crate `{name}` listed in VENDOR.toml but absent"
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    const SHA: &str = "4eb2fb5c8ed30a580acf24234fffb2cb6256e335";

    fn fixture(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("xtask-vendor-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let repo = root.join("vendor/gridcraft");
        fs::create_dir_all(repo.join("crates/core")).unwrap();
        for f in ["LICENSE-MIT", "LICENSE-APACHE", "NOTICE"] {
            fs::write(repo.join(f), "x").unwrap();
        }
        write_manifest(&root, SHA, &["core"]);
        fs::write(root.join("NOTICE"), format!("vendor/gridcraft @ {SHA}\n")).unwrap();
        root
    }

    fn write_manifest(root: &Path, sha: &str, crates: &[&str]) {
        let list = crates
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let body = format!(
            "upstream = \"https://github.com/storytold/gridcraft\"\nsha = \"{sha}\"\ncrates = [{list}]\n"
        );
        fs::write(root.join("vendor/gridcraft/VENDOR.toml"), body).unwrap();
    }

    #[test]
    fn clean_vendor_dir_passes() {
        let root = fixture("clean");
        assert!(vendor_check(&root).is_empty());
    }

    #[test]
    fn missing_vendor_dir_passes() {
        assert!(vendor_check(Path::new("/nonexistent-xtask-root")).is_empty());
    }

    #[test]
    fn missing_notice_fails() {
        let root = fixture("notice");
        fs::remove_file(root.join("vendor/gridcraft/NOTICE")).unwrap();
        assert_eq!(vendor_check(&root), ["gridcraft: missing NOTICE"]);
    }

    #[test]
    fn unlisted_crate_fails() {
        let root = fixture("unlisted");
        fs::create_dir_all(root.join("vendor/gridcraft/crates/formula")).unwrap();
        assert_eq!(
            vendor_check(&root),
            ["gridcraft: crate `formula` present but not listed in VENDOR.toml"]
        );
    }

    #[test]
    fn listed_but_absent_crate_fails() {
        let root = fixture("absent");
        write_manifest(&root, SHA, &["core", "formula"]);
        assert_eq!(
            vendor_check(&root),
            ["gridcraft: crate `formula` listed in VENDOR.toml but absent"]
        );
    }

    #[test]
    fn ui_egui_present_fails() {
        let root = fixture("egui");
        fs::create_dir_all(root.join("vendor/gridcraft/crates/ui-egui")).unwrap();
        write_manifest(&root, SHA, &["core", "ui-egui"]);
        assert_eq!(
            vendor_check(&root),
            ["gridcraft: forbidden path `crates/ui-egui` present"]
        );
    }

    #[test]
    fn sha_not_in_root_notice_fails() {
        let root = fixture("notice-sha");
        fs::write(
            root.join("NOTICE"),
            format!("vendor/gridcraft @ {}\n", "0".repeat(40)),
        )
        .unwrap();
        assert_eq!(
            vendor_check(&root),
            [format!(
                "gridcraft: root NOTICE does not list vendor/gridcraft at sha {SHA}"
            )]
        );
    }

    #[test]
    fn bad_sha_fails() {
        let root = fixture("sha");
        write_manifest(&root, "main", &["core"]);
        assert_eq!(
            vendor_check(&root),
            [
                "gridcraft: bad sha `main` (need 40 hex chars)",
                "gridcraft: root NOTICE does not list vendor/gridcraft at sha main"
            ]
        );
    }
}
