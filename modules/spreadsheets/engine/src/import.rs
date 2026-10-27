use gridcraft_core::Value;
use gridcraft_model::{
    Cell, Workbook,
    style::{Color, HAlign, PatternType, Underline},
};
use serde_json::{Map, Value as Json, json};

use crate::{
    limits::{
        MAX_CELLS, MAX_COL, MAX_COL_WIDTH, MAX_INPUT_CHARS, MAX_NAME_CHARS, MAX_ROW, MAX_SHEETS,
        MAX_XLSX_BYTES, MIN_COL_WIDTH,
    },
    syntax,
};

/// A workbook read from a file.
#[derive(Debug)]
pub struct Imported {
    /// Workbook document in the shape the web grid stores.
    pub workbook: Json,
    /// What was dropped or approximated (limits, dates, unsupported parts), capped at 20 lines.
    pub warnings: Vec<String>,
}

/// Why a file could not be imported.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    /// The upload exceeds [`MAX_XLSX_BYTES`].
    #[error("file is too large")]
    TooLarge,
    /// Not a readable `.xlsx` workbook.
    #[error("not a valid xlsx file: {0}")]
    Invalid(String),
}

const MAX_WARNINGS: usize = 20;

struct Report {
    lines: Vec<String>,
}

impl Report {
    fn add(&mut self, line: impl Into<String>) {
        let line = line.into();
        if self.lines.len() < MAX_WARNINGS && !self.lines.contains(&line) {
            self.lines.push(line);
        }
    }
}

/// Reads an `.xlsx` package into the workbook document.
///
/// # Errors
/// [`ImportError`] when the file is too large or not a workbook.
pub fn import_xlsx(bytes: &[u8]) -> Result<Imported, ImportError> {
    if bytes.len() > MAX_XLSX_BYTES {
        return Err(ImportError::TooLarge);
    }
    let (book, read) = gridcraft_xlsx::read_xlsx(bytes).map_err(|e| match e {
        gridcraft_xlsx::IoError::TooLarge(_) => ImportError::TooLarge,
        other => ImportError::Invalid(other.to_string()),
    })?;
    let mut report = Report { lines: Vec::new() };
    for warning in read.warnings.iter().take(MAX_WARNINGS) {
        report.add(warning.clone());
    }

    let mut sheets = Vec::new();
    let mut dates = 0usize;
    let mut cells_left = MAX_CELLS;
    for (index, sheet) in book.sheets.iter().enumerate() {
        if index == MAX_SHEETS {
            report.add(format!("only the first {MAX_SHEETS} sheets were imported"));
            break;
        }
        let mut cells = Map::new();
        for (at, cell) in sheet.cells.iter() {
            if at.row >= MAX_ROW || at.col >= MAX_COL {
                report.add(format!(
                    "cells beyond row {MAX_ROW} or column {MAX_COL} were dropped"
                ));
                continue;
            }
            if cells_left == 0 {
                report.add(format!("only the first {MAX_CELLS} cells were imported"));
                break;
            }
            if let Some(json) = convert_cell(&book, cell, &mut dates, &mut report) {
                cells.insert(format!("{},{}", at.row, at.col), json);
                cells_left -= 1;
            }
        }
        let name: String = sheet.name.chars().take(MAX_NAME_CHARS).collect();
        let mut out = json!({
            "id": format!("s{}", index + 1),
            "name": if name.trim().is_empty() { format!("Sheet{}", index + 1) } else { name },
            "cells": cells,
        });
        let widths = column_widths(sheet);
        if !widths.is_empty() {
            out["cols"] = Json::Object(widths);
        }
        sheets.push(out);
    }
    if sheets.is_empty() {
        sheets.push(json!({ "id": "s1", "name": "Sheet1", "cells": {} }));
    }
    if dates > 0 {
        report.add(format!(
            "{dates} date or time cells were imported as plain numbers"
        ));
    }
    Ok(Imported {
        workbook: json!({ "sheets": sheets }),
        warnings: report.lines,
    })
}

fn column_widths(sheet: &gridcraft_model::Sheet) -> Map<String, Json> {
    let mut widths = Map::new();
    for (&col, info) in &sheet.cols {
        let (true, Some(size)) = (col < MAX_COL, info.size) else {
            continue;
        };
        if (size - sheet.default_col_width).abs() < 1.0 {
            continue;
        }
        let clamped = size.clamp(MIN_COL_WIDTH, MAX_COL_WIDTH).round();
        widths.insert(col.to_string(), json!(whole_pixels(clamped)));
    }
    widths
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the width is clamped to 32..=800 and rounded, so it fits a u64 exactly"
)]
const fn whole_pixels(width: f32) -> u64 {
    width as u64
}

fn convert_cell(
    book: &Workbook,
    cell: &Cell,
    dates: &mut usize,
    report: &mut Report,
) -> Option<Json> {
    let style = book.styles.get(cell.style);
    let input = cell.formula.as_ref().map_or_else(
        || value_text(&cell.value),
        |formula| format!("={}", syntax::to_stored(&formula.text)),
    );
    if input.chars().count() > MAX_INPUT_CHARS {
        report.add(format!(
            "cells longer than {MAX_INPUT_CHARS} characters were dropped"
        ));
        return None;
    }
    let mut out = Map::new();
    if style.font.bold {
        out.insert("bold".to_owned(), json!(true));
    }
    if style.font.italic {
        out.insert("italic".to_owned(), json!(true));
    }
    if style.font.underline != Underline::None {
        out.insert("underline".to_owned(), json!(true));
    }
    match style.align.h {
        HAlign::Left => out.insert("align".to_owned(), json!("left")),
        HAlign::Center | HAlign::CenterAcross => out.insert("align".to_owned(), json!("center")),
        HAlign::Right => out.insert("align".to_owned(), json!("right")),
        _ => None,
    };
    if let Some(color) = color_hex(book, style.font.color) {
        out.insert("color".to_owned(), json!(color));
    }
    if style.fill.pattern == PatternType::Solid
        && let Some(fill) = color_hex(book, style.fill.fg)
    {
        out.insert("fill".to_owned(), json!(fill));
    }
    let code = style.num_fmt.as_str();
    if is_date_format(code) {
        if !cell.value.is_empty() {
            *dates += 1;
        }
    } else if let Some(format) = format_name(code) {
        out.insert("format".to_owned(), json!(format));
    }
    if input.is_empty() && !out.contains_key("fill") {
        return None;
    }
    out.insert("input".to_owned(), json!(input));
    Some(Json::Object(out))
}

fn color_hex(book: &Workbook, color: Color) -> Option<String> {
    color.hex(&book.theme).map(|hex| hex.to_lowercase())
}

fn value_text(value: &Value) -> String {
    match value {
        Value::Empty => String::new(),
        Value::Text(text) => text.to_string(),
        Value::Bool(flag) => if *flag { "TRUE" } else { "FALSE" }.to_owned(),
        other => other.display(),
    }
}

/// Strips quoted text and bracketed parts (`[$-409]`, `[Red]`) from a number format code.
fn bare_code(code: &str) -> String {
    let mut out = String::new();
    let (mut quoted, mut bracket) = (false, false);
    for c in code.chars() {
        match c {
            '"' => quoted = !quoted,
            '[' if !quoted => bracket = true,
            ']' if !quoted => bracket = false,
            c if !quoted && !bracket => out.push(c),
            _ => {}
        }
    }
    out
}

fn is_date_format(code: &str) -> bool {
    bare_code(code)
        .chars()
        .any(|c| matches!(c.to_ascii_lowercase(), 'y' | 'd' | 'h' | 's'))
}

fn format_name(code: &str) -> Option<&'static str> {
    if code == "General" || code == "@" {
        return None;
    }
    if code.contains('%') {
        return Some("percent");
    }
    if code.contains("[$")
        || code
            .chars()
            .any(|c| matches!(c, '$' | '€' | '£' | '₺' | '¥'))
    {
        return Some("currency");
    }
    bare_code(code)
        .chars()
        .any(|c| c == '0' || c == '#')
        .then_some("number")
}
