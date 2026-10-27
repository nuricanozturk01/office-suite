//! Gzip size budgets for built web assets (`budgets.toml`).

use std::path::Path;

/// One entry of `budgets.toml`.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct Budget {
    pub(crate) name: String,
    pub(crate) glob: String,
    pub(crate) max_gzip_bytes: u64,
    #[serde(default)]
    pub(crate) optional: bool,
}

#[derive(serde::Deserialize)]
struct BudgetFile {
    budget: Vec<Budget>,
}

/// Reads `<root>/budgets.toml`.
pub(crate) fn load(root: &Path) -> anyhow::Result<Vec<Budget>> {
    use anyhow::Context;
    let raw = std::fs::read_to_string(root.join("budgets.toml")).context("read budgets.toml")?;
    Ok(toml::from_str::<BudgetFile>(&raw)
        .context("parse budgets.toml")?
        .budget)
}

/// Returns one message per violated budget; paths in `glob` are relative to `root`.
pub(crate) fn budget_violations(root: &Path, budgets: &[Budget]) -> Vec<String> {
    let mut out = Vec::new();
    for b in budgets {
        let (dir, pattern) = b.glob.rsplit_once('/').unwrap_or((".", &b.glob));
        let mut files: Vec<String> = std::fs::read_dir(root.join(dir))
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| wildcard_match(pattern, name))
            .collect();
        files.sort();
        if files.is_empty() {
            if !b.optional {
                out.push(format!("{}: no file matches {}", b.name, b.glob));
            }
            continue;
        }
        for name in files {
            let rel = format!("{dir}/{name}");
            match gzip_size(&root.join(&rel)) {
                Ok(size) if size > b.max_gzip_bytes => out.push(format!(
                    "{}: {rel} is {size} B gzip, limit {} B",
                    b.name, b.max_gzip_bytes
                )),
                Ok(_) => {}
                Err(err) => out.push(format!("{}: {rel}: {err}", b.name)),
            }
        }
    }
    out
}

fn gzip_size(path: &Path) -> std::io::Result<u64> {
    use std::io::Write;

    use flate2::Compression;
    use flate2::write::GzEncoder;

    let mut enc = GzEncoder::new(Vec::new(), Compression::best());
    enc.write_all(&std::fs::read(path)?)?;
    Ok(enc.finish()?.len() as u64)
}

/// Matches `name` against a pattern where `*` stands for any (possibly empty) run of characters.
fn wildcard_match(pattern: &str, name: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = name.strip_prefix(first) else {
        return false;
    };
    let tail: Vec<&str> = parts.collect();
    let Some((last, middle)) = tail.split_last() else {
        return rest.is_empty();
    };
    for part in middle {
        let Some(at) = rest.find(part) else {
            return false;
        };
        rest = &rest[at + part.len()..];
    }
    rest.ends_with(last)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    fn root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("xtask-budget-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("dist")).unwrap();
        root
    }

    fn budget(glob: &str, max: u64, optional: bool) -> Budget {
        Budget {
            name: "js".to_owned(),
            glob: glob.to_owned(),
            max_gzip_bytes: max,
            optional,
        }
    }

    /// Incompressible bytes (LCG), so the gzip size is close to the input size.
    fn noise(len: usize) -> Vec<u8> {
        let mut x: u32 = 12345;
        (0..len)
            .map(|_| {
                x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (x >> 24) as u8
            })
            .collect()
    }

    #[test]
    fn under_limit_passes() {
        let r = root("under");
        fs::write(r.join("dist/index-a1.js"), "console.log(1)").unwrap();
        assert!(budget_violations(&r, &[budget("dist/index-*.js", 1000, false)]).is_empty());
    }

    #[test]
    fn over_limit_fails_with_exact_message_prefix() {
        let r = root("over");
        fs::write(r.join("dist/index-a1.js"), noise(5000)).unwrap();
        let v = budget_violations(&r, &[budget("dist/index-*.js", 1000, false)]);
        assert_eq!(v.len(), 1);
        assert!(v[0].starts_with("js: dist/index-a1.js is "), "{}", v[0]);
        assert!(v[0].ends_with(" B gzip, limit 1000 B"), "{}", v[0]);
    }

    #[test]
    fn missing_required_glob_fails() {
        let r = root("missing");
        assert_eq!(
            budget_violations(&r, &[budget("dist/index-*.js", 1000, false)]),
            ["js: no file matches dist/index-*.js"]
        );
    }

    #[test]
    fn missing_optional_glob_is_ignored() {
        let r = root("optional");
        assert!(budget_violations(&r, &[budget("dist/*formula*.wasm", 1000, true)]).is_empty());
    }
}
