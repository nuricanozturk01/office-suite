//! Three-way merge of `.docx` files: two people edited one document concurrently.
//!
//! Paragraphs and tables (the body's blocks) are the unit: edits to different blocks are all
//! kept, edits to the same block are a conflict. Everything outside the body (styles, headers,
//! settings) comes from whichever side changed it (ours if both did); media and comments only
//! the other side has are carried over so no block loses what it refers to.

use storage_contracts::{MergeError, PackageMerger};

/// Merges `.docx` files; see the module docs.
#[derive(Debug, Default, Clone, Copy)]
pub struct DocxMerger;

impl PackageMerger for DocxMerger {
    fn handles(&self, name: &str) -> bool {
        name.rsplit_once('.')
            .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("docx"))
    }

    fn merge(&self, base: &[u8], ours: &[u8], theirs: &[u8]) -> Result<Vec<u8>, MergeError> {
        let read = |bytes: &[u8]| wordcraft_docx::read(bytes).map_err(|_| MergeError::Unreadable);
        let (base, ours, theirs) = (read(base)?, read(ours)?, read(theirs)?);
        let body = merge3::merge3(&base.body, &ours.body, &theirs.body)
            .map_err(|_| MergeError::Conflict)?;
        // Everything but the body (styles, headers, settings) comes from whoever changed it;
        // when only the other side did, that is `theirs`.
        let mut ours_outside_body = ours.clone();
        ours_outside_body.body.clone_from(&base.body);
        let (mut merged, other) = if ours_outside_body == base {
            (theirs, ours)
        } else {
            (ours, theirs)
        };
        // Blocks from either side keep what they refer to.
        for (key, media) in other.media {
            merged.media.entry(key).or_insert(media);
        }
        for (id, comment) in other.comments {
            merged.comments.entry(id).or_insert(comment);
        }
        merged.body = body;
        wordcraft_docx::write(&merged).map_err(|_| MergeError::Unreadable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{export_docx, import_docx};

    fn docx(paragraphs: &[&str]) -> Vec<u8> {
        docx_titled(paragraphs, "T")
    }

    fn docx_titled(paragraphs: &[&str], title: &str) -> Vec<u8> {
        let content: Vec<serde_json::Value> = paragraphs
            .iter()
            .map(|text| {
                serde_json::json!({"type": "paragraph", "content": [{"type": "text", "text": text}]})
            })
            .collect();
        let doc = serde_json::json!({"type": "doc", "content": content});
        export_docx(&doc, title).unwrap()
    }

    fn texts(bytes: &[u8]) -> Vec<String> {
        let imported = import_docx(bytes).unwrap();
        imported.content["content"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["content"][0]["text"].as_str().unwrap_or("").to_string())
            .collect()
    }

    #[test]
    fn edits_to_different_paragraphs_are_both_kept() {
        let base = docx(&["alpha", "beta", "gamma"]);
        let ours = docx(&["alpha", "beta edited by me", "gamma"]);
        let theirs = docx(&["alpha", "beta", "gamma", "added by them"]);
        let merged = DocxMerger.merge(&base, &ours, &theirs).unwrap();
        assert_eq!(
            texts(&merged),
            ["alpha", "beta edited by me", "gamma", "added by them"]
        );
    }

    #[test]
    fn settings_changed_only_by_the_other_side_are_kept() {
        let base = docx(&["alpha", "beta"]);
        let ours = docx(&["alpha", "beta edited by me"]);
        let theirs = docx_titled(&["alpha", "beta"], "Their title");
        let merged = DocxMerger.merge(&base, &ours, &theirs).unwrap();
        assert_eq!(texts(&merged), ["alpha", "beta edited by me"]);
        let title = wordcraft_docx::read(&merged).unwrap().core.title;
        assert_eq!(title, "Their title");
    }

    #[test]
    fn the_same_paragraph_edited_twice_conflicts() {
        let base = docx(&["alpha", "beta"]);
        let ours = docx(&["alpha", "mine"]);
        let theirs = docx(&["alpha", "theirs"]);
        assert_eq!(
            DocxMerger.merge(&base, &ours, &theirs),
            Err(MergeError::Conflict)
        );
    }

    #[test]
    fn garbage_is_unreadable() {
        let base = docx(&["a"]);
        assert_eq!(
            DocxMerger.merge(&base, b"nope", &base),
            Err(MergeError::Unreadable)
        );
    }

    #[test]
    fn only_docx_names_are_handled() {
        assert!(DocxMerger.handles("Report.DOCX"));
        assert!(!DocxMerger.handles("Sheet.xlsx"));
        assert!(!DocxMerger.handles("docx"));
    }
}
