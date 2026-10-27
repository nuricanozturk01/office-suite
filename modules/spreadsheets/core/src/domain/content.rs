//! Sparse workbook document stored as JSON. The shape is the contract with the web grid.

use std::collections::HashSet;

use serde_json::{Map, Value};
use thiserror::Error;

const MAX_BYTES: usize = 1_048_576;
const MAX_SHEETS: usize = 20;
const MAX_CELLS: usize = 20_000;
const MAX_NAME: usize = 100;
const MAX_INPUT: usize = 8_000;
const MAX_ROW: u64 = 5_000;
const MAX_COL: u64 = 200;
const MIN_COL_WIDTH: u64 = 32;
const MAX_COL_WIDTH: u64 = 800;

const CELL_FLAGS: &[&str] = &["bold", "italic", "underline"];
const ALIGNS: &[&str] = &["left", "center", "right"];
const FORMATS: &[&str] = &["number", "percent", "currency"];

/// Rejected workbook document.
#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum ContentError {
    /// Shape does not match the workbook schema.
    #[error("workbook is invalid")]
    Invalid,
    /// Serialized document exceeds the size cap.
    #[error("workbook is too large")]
    TooLarge,
}

/// Validated workbook JSON.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkbookContent(Value);

impl WorkbookContent {
    /// Empty workbook with a single sheet.
    #[must_use]
    pub(crate) fn empty(sheet_id: &str, sheet_name: &str) -> Self {
        Self(serde_json::json!({
            "sheets": [{ "id": sheet_id, "name": sheet_name, "cells": {} }]
        }))
    }

    /// Parses a client-supplied workbook.
    ///
    /// # Errors
    /// Returns [`ContentError`] when the document is too large or malformed.
    pub(crate) fn parse(value: Value) -> Result<Self, ContentError> {
        let bytes = serde_json::to_vec(&value).map_err(|_| ContentError::Invalid)?;
        if bytes.len() > MAX_BYTES {
            return Err(ContentError::TooLarge);
        }
        validate(&value)?;
        Ok(Self(value))
    }

    /// Borrowed JSON value.
    #[must_use]
    pub(crate) const fn as_value(&self) -> &Value {
        &self.0
    }
}

fn validate(value: &Value) -> Result<(), ContentError> {
    let root = value.as_object().ok_or(ContentError::Invalid)?;
    if root.len() != 1 || !root.contains_key("sheets") {
        return Err(ContentError::Invalid);
    }
    let sheets = root["sheets"].as_array().ok_or(ContentError::Invalid)?;
    if sheets.is_empty() || sheets.len() > MAX_SHEETS {
        return Err(ContentError::Invalid);
    }
    let mut ids = HashSet::new();
    for sheet in sheets {
        validate_sheet(sheet, &mut ids)?;
    }
    Ok(())
}

fn validate_sheet(value: &Value, ids: &mut HashSet<String>) -> Result<(), ContentError> {
    let sheet = value.as_object().ok_or(ContentError::Invalid)?;
    let id = required_str(sheet, "id")?;
    if id.is_empty() || id.chars().count() > 64 || !ids.insert(id.to_owned()) {
        return Err(ContentError::Invalid);
    }
    let name = required_str(sheet, "name")?;
    if name.trim().is_empty() || name.chars().count() > MAX_NAME {
        return Err(ContentError::Invalid);
    }
    let cells = sheet
        .get("cells")
        .and_then(Value::as_object)
        .ok_or(ContentError::Invalid)?;
    if cells.len() > MAX_CELLS {
        return Err(ContentError::TooLarge);
    }
    for (key, cell) in cells {
        validate_cell(key, cell)?;
    }
    if let Some(cols) = sheet.get("cols") {
        validate_cols(cols)?;
    }
    let allowed = ["id", "name", "cells", "cols"];
    if sheet.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(ContentError::Invalid);
    }
    Ok(())
}

fn validate_cell(key: &str, value: &Value) -> Result<(), ContentError> {
    let (row, col) = key.split_once(',').ok_or(ContentError::Invalid)?;
    let row: u64 = row.parse().map_err(|_| ContentError::Invalid)?;
    let col: u64 = col.parse().map_err(|_| ContentError::Invalid)?;
    if row >= MAX_ROW || col >= MAX_COL {
        return Err(ContentError::Invalid);
    }
    let cell = value.as_object().ok_or(ContentError::Invalid)?;
    let input = required_str(cell, "input")?;
    if input.chars().count() > MAX_INPUT {
        return Err(ContentError::Invalid);
    }
    for flag in CELL_FLAGS {
        if let Some(value) = cell.get(*flag)
            && !value.is_boolean()
        {
            return Err(ContentError::Invalid);
        }
    }
    if let Some(align) = cell.get("align") {
        let align = align.as_str().ok_or(ContentError::Invalid)?;
        if !ALIGNS.contains(&align) {
            return Err(ContentError::Invalid);
        }
    }
    for key in ["color", "fill"] {
        if let Some(color) = cell.get(key)
            && !color.as_str().is_some_and(is_hex_color)
        {
            return Err(ContentError::Invalid);
        }
    }
    if let Some(format) = cell.get("format") {
        let format = format.as_str().ok_or(ContentError::Invalid)?;
        if !FORMATS.contains(&format) {
            return Err(ContentError::Invalid);
        }
    }
    let allowed = [
        "input",
        "bold",
        "italic",
        "underline",
        "align",
        "color",
        "fill",
        "format",
    ];
    if cell.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(ContentError::Invalid);
    }
    Ok(())
}

fn validate_cols(value: &Value) -> Result<(), ContentError> {
    let cols = value.as_object().ok_or(ContentError::Invalid)?;
    for (key, width) in cols {
        let index: u64 = key.parse().map_err(|_| ContentError::Invalid)?;
        let width = width.as_u64().ok_or(ContentError::Invalid)?;
        if index >= MAX_COL || !(MIN_COL_WIDTH..=MAX_COL_WIDTH).contains(&width) {
            return Err(ContentError::Invalid);
        }
    }
    Ok(())
}

fn required_str<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a str, ContentError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or(ContentError::Invalid)
}

fn is_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    hex.len() == 6 && hex.chars().all(|char| char.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet() -> Value {
        serde_json::json!({
            "sheets": [{
                "id": "s1",
                "name": "Sayfa1",
                "cells": { "0,1": { "input": "=TOPLA(A1:A3)", "bold": true, "format": "number" } },
                "cols": { "1": 140 }
            }]
        })
    }

    #[test]
    fn accepts_a_sparse_sheet() {
        assert!(WorkbookContent::parse(sheet()).is_ok());
    }

    #[test]
    fn rejects_unknown_fields_and_bad_colors() {
        let mut value = sheet();
        value["sheets"][0]["cells"]["0,1"]["color"] = Value::String("red".to_owned());
        assert_eq!(WorkbookContent::parse(value), Err(ContentError::Invalid));
    }

    #[test]
    fn rejects_an_empty_sheet_list() {
        assert_eq!(
            WorkbookContent::parse(serde_json::json!({ "sheets": [] })),
            Err(ContentError::Invalid)
        );
    }
}
