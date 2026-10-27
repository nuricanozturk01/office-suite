//! A calculation session: one workbook held in the vendored engine, recalculated incrementally.
//! The web grid drives it through the WASM wrapper; native code uses it in tests.

use gridcraft_calc::Calc;
use gridcraft_core::{CellError, CellRef, Value};
use gridcraft_model::Workbook;
use serde_json::Value as Json;

use crate::{
    limits::{MAX_COL, MAX_INPUT_CHARS, MAX_ROW},
    model::{ModelError, build_cell, workbook_from_json},
};

/// What a cell shows after calculation. Arrays never appear: spilled cells hold their own value.
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    /// Nothing in the cell.
    Empty,
    /// A number.
    Number(f64),
    /// Text.
    Text(String),
    /// A boolean.
    Bool(bool),
    /// An error such as `#DIV/0!`.
    Error(&'static str),
}

impl From<Value> for CellValue {
    fn from(value: Value) -> Self {
        match value {
            Value::Empty => Self::Empty,
            Value::Number(n) => Self::Number(n),
            Value::Text(text) => Self::Text(text.to_string()),
            Value::Bool(flag) => Self::Bool(flag),
            Value::Error(err) => Self::Error(err.as_str()),
            Value::Array(array) => array.get(0, 0).cloned().map_or(Self::Empty, Self::from),
        }
    }
}

/// Why an edit was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    /// The sheet does not exist.
    #[error("no such sheet")]
    NoSuchSheet,
    /// The cell lies outside the supported grid.
    #[error("cell is out of bounds")]
    OutOfBounds,
    /// The input exceeds the length limit.
    #[error("input is too long")]
    TooLong,
}

/// One edit: `(row, column, input)`. Inputs starting with `=` are formulas in the stored syntax.
pub type Edit<'a> = (u32, u32, &'a str);

/// A workbook plus its dependency graph.
#[derive(Debug)]
pub struct Session {
    book: Workbook,
    calc: Calc,
}

impl Session {
    /// Loads a stored workbook document and calculates every formula.
    ///
    /// # Errors
    /// [`ModelError`] when the document is malformed.
    pub fn from_json(workbook: &Json) -> Result<Self, ModelError> {
        let mut book = workbook_from_json(workbook)?;
        let mut calc = Calc::new();
        calc.recalc_all(&mut book);
        mark_unparsable(&mut book);
        Ok(Self { book, calc })
    }

    /// Number of sheets.
    #[must_use]
    pub const fn sheet_count(&self) -> usize {
        self.book.sheets.len()
    }

    /// The calculated value at a cell; out-of-range positions are empty.
    #[must_use]
    pub fn value_at(&self, sheet: usize, row: u32, col: u32) -> CellValue {
        self.book.sheet(sheet).map_or(CellValue::Empty, |s| {
            CellValue::from(s.value(CellRef::new(row, col)))
        })
    }

    /// Applies edits to one sheet and recalculates everything that depends on them.
    ///
    /// # Errors
    /// [`EditError`] when the sheet or a cell is invalid; nothing is applied then.
    pub fn set_inputs(&mut self, sheet: usize, edits: &[Edit<'_>]) -> Result<(), EditError> {
        if self.book.sheet(sheet).is_none() {
            return Err(EditError::NoSuchSheet);
        }
        for &(row, col, input) in edits {
            if row >= MAX_ROW || col >= MAX_COL {
                return Err(EditError::OutOfBounds);
            }
            if input.chars().count() > MAX_INPUT_CHARS {
                return Err(EditError::TooLong);
            }
        }
        let mut changed = Vec::with_capacity(edits.len());
        for &(row, col, input) in edits {
            let at = CellRef::new(row, col);
            let Some(target) = self.book.sheet_mut(sheet) else {
                return Err(EditError::NoSuchSheet);
            };
            if input.trim().is_empty() {
                target.remove_cell(at);
            } else {
                let style = target.style_id(at);
                let mut cell = build_cell(input);
                cell.style = style;
                target.set_cell(at, cell);
            }
            changed.push((sheet, at));
        }
        self.calc.cells_changed(&mut self.book, &changed);
        Ok(())
    }
}

/// A formula that does not parse is never evaluated; it shows `#NAME?` like an unknown function
/// (`Calc::cells_changed` does the same for edits).
fn mark_unparsable(book: &mut Workbook) {
    for index in 0..book.sheets.len() {
        let broken: Vec<CellRef> = book
            .sheet(index)
            .map(|sheet| {
                sheet
                    .cells
                    .iter()
                    .filter(|(_, cell)| cell.formula.as_ref().is_some_and(|f| f.expr().is_none()))
                    .map(|(at, _)| at)
                    .collect()
            })
            .unwrap_or_default();
        for at in broken {
            if let Some(cell) = book.sheet_mut(index).and_then(|s| s.cells.get_mut(at)) {
                cell.value = Value::Error(CellError::Name);
            }
        }
    }
}
