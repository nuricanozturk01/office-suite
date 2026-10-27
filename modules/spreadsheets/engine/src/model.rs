//! Workbook JSON to the vendored gridcraft model. Shared by `.xlsx` export and the calculation
//! session, so neither needs the other.

use std::sync::Arc;

use gridcraft_core::{CellRef, Value};
use gridcraft_model::{
    Cell, Formula, Sheet, Workbook,
    features::LineInfo,
    style::{Alignment, Color, Fill, Font, HAlign, NumFmt, Style, Underline},
};
use serde::Deserialize;
use serde_json::Value as Json;

use crate::{limits::MAX_COL, syntax};

/// The workbook document does not have the expected shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("workbook document is invalid")]
pub struct ModelError;

#[derive(Deserialize)]
struct Doc {
    sheets: Vec<SheetDoc>,
}

#[derive(Deserialize)]
struct SheetDoc {
    name: String,
    #[serde(default)]
    cells: std::collections::BTreeMap<String, CellDoc>,
    #[serde(default)]
    cols: std::collections::BTreeMap<String, f32>,
}

#[derive(Deserialize)]
struct CellDoc {
    input: String,
    #[serde(default)]
    bold: bool,
    #[serde(default)]
    italic: bool,
    #[serde(default)]
    underline: bool,
    align: Option<String>,
    color: Option<String>,
    fill: Option<String>,
    format: Option<String>,
}

/// Builds the engine model from the stored workbook document.
///
/// # Errors
/// [`ModelError`] when the document is malformed or has no sheet.
pub fn workbook_from_json(workbook: &Json) -> Result<Workbook, ModelError> {
    let doc: Doc = serde_json::from_value(workbook.clone()).map_err(|_| ModelError)?;
    let mut book = Workbook::new();
    book.sheets.clear();
    for sheet_doc in &doc.sheets {
        let mut sheet = Sheet::new(sheet_doc.name.clone());
        for (key, cell_doc) in &sheet_doc.cells {
            let at = parse_key(key).ok_or(ModelError)?;
            let style = book.styles.intern(style_of(cell_doc));
            let mut cell = build_cell(&cell_doc.input);
            cell.style = style;
            sheet.set_cell(at, cell);
        }
        for (key, &width) in &sheet_doc.cols {
            let col: u32 = key.parse().map_err(|_| ModelError)?;
            if col < MAX_COL {
                sheet.cols.insert(
                    col,
                    LineInfo {
                        size: Some(width),
                        ..LineInfo::default()
                    },
                );
            }
        }
        book.sheets.push(Arc::new(sheet));
    }
    if book.sheets.is_empty() {
        return Err(ModelError);
    }
    Ok(book)
}

pub(crate) fn parse_key(key: &str) -> Option<CellRef> {
    let (row, col) = key.split_once(',')?;
    Some(CellRef::new(row.parse().ok()?, col.parse().ok()?))
}

pub(crate) fn build_cell(input: &str) -> Cell {
    if let Some(body) = input.strip_prefix('=') {
        return Cell::formula(Formula::new(&syntax::to_canonical(body)));
    }
    Cell::value(literal(input))
}

/// A typed constant: number, percentage, boolean or text (empty input stays empty).
pub(crate) fn literal(input: &str) -> Value {
    let text = input.trim();
    if text.is_empty() {
        return Value::Empty;
    }
    // Lower-casing is Unicode-aware, so `DOĞRU` and `YANLIŞ` match too.
    match text.to_lowercase().as_str() {
        "true" | "doğru" => return Value::Bool(true),
        "false" | "yanlış" => return Value::Bool(false),
        _ => {}
    }
    if let Some(number) = text.strip_suffix('%').and_then(parse_number) {
        return Value::number(number / 100.0);
    }
    parse_number(text).map_or_else(|| Value::Text(Arc::from(input)), Value::number)
}

/// `1234.5` or `1234,5` (one comma between digits), as the grid accepts them.
fn parse_number(text: &str) -> Option<f64> {
    let normalized = if text.matches(',').count() == 1 && !text.contains('.') {
        text.replace(',', ".")
    } else {
        text.to_owned()
    };
    let plain = normalized.strip_prefix('-').unwrap_or(&normalized);
    if plain.is_empty() || !plain.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return None;
    }
    normalized.parse::<f64>().ok().filter(|n| n.is_finite())
}

fn style_of(doc: &CellDoc) -> Style {
    let font = Font {
        bold: doc.bold,
        italic: doc.italic,
        underline: if doc.underline {
            Underline::Single
        } else {
            Underline::None
        },
        color: doc
            .color
            .as_deref()
            .and_then(Color::from_hex)
            .unwrap_or(Color::Auto),
        ..Font::default()
    };
    let fill = doc
        .fill
        .as_deref()
        .and_then(Color::from_hex)
        .map_or_else(Fill::default, Fill::solid);
    let align = Alignment {
        h: match doc.align.as_deref() {
            Some("left") => HAlign::Left,
            Some("center") => HAlign::Center,
            Some("right") => HAlign::Right,
            _ => HAlign::General,
        },
        ..Alignment::default()
    };
    let num_fmt = doc
        .format
        .as_deref()
        .and_then(number_format_code)
        .map_or_else(NumFmt::default, NumFmt::new);
    Style {
        font,
        fill,
        align,
        num_fmt,
        ..Style::default()
    }
}

fn number_format_code(format: &str) -> Option<&'static str> {
    match format {
        "number" => Some("#,##0.00"),
        "percent" => Some("0.00%"),
        "currency" => Some("\"$\"#,##0.00"),
        _ => None,
    }
}
