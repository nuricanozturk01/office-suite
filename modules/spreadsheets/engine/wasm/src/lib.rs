//! Browser binding of [`spreadsheets_engine::Session`]: the grid keeps one [`Engine`], pushes
//! edits into it and reads calculated values back. Values cross the boundary as plain objects
//! shaped like the web `CellValue` (`{ kind, value }`).

use js_sys::{Object, Reflect};
use spreadsheets_engine::{CellValue, Session};
use wasm_bindgen::prelude::*;

/// A workbook being calculated.
#[wasm_bindgen]
#[derive(Debug)]
pub struct Engine {
    session: Session,
}

fn load_session(workbook_json: &str) -> Result<Session, JsError> {
    let document: serde_json::Value = serde_json::from_str(workbook_json)
        .map_err(|_| JsError::new("workbook is not valid JSON"))?;
    Session::from_json(&document).map_err(|err| JsError::new(&err.to_string()))
}

fn field(object: &Object, key: &str, value: &JsValue) {
    // Setting a property on a fresh plain object cannot fail.
    let _ = Reflect::set(object, &JsValue::from_str(key), value);
}

#[wasm_bindgen]
impl Engine {
    /// Loads a stored workbook document (`{ "sheets": [...] }`) and calculates it.
    ///
    /// # Errors
    /// Fails when the text is not a workbook document.
    #[wasm_bindgen(constructor)]
    pub fn new(workbook_json: &str) -> Result<Self, JsError> {
        Ok(Self {
            session: load_session(workbook_json)?,
        })
    }

    /// Replaces the whole workbook (sheet added, renamed or removed, undo, redo, reload).
    ///
    /// # Errors
    /// Fails when the text is not a workbook document; the previous workbook is kept then.
    pub fn load(&mut self, workbook_json: &str) -> Result<(), JsError> {
        self.session = load_session(workbook_json)?;
        Ok(())
    }

    /// Number of sheets.
    #[wasm_bindgen(js_name = sheetCount)]
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "`wasm_bindgen` rejects `const fn` exports"
    )]
    pub fn sheet_count(&self) -> usize {
        self.session.sheet_count()
    }

    /// The calculated value of a cell as `{ kind: 'empty' | 'number' | 'text' | 'bool' | 'error', value }`.
    #[wasm_bindgen(js_name = valueAt)]
    #[must_use]
    pub fn value_at(&self, sheet: u32, row: u32, col: u32) -> JsValue {
        let object = Object::new();
        let (kind, value) = match self.session.value_at(sheet as usize, row, col) {
            CellValue::Empty => ("empty", JsValue::UNDEFINED),
            CellValue::Number(n) => ("number", JsValue::from_f64(n)),
            CellValue::Text(text) => ("text", JsValue::from_str(&text)),
            CellValue::Bool(flag) => ("bool", JsValue::from_bool(flag)),
            CellValue::Error(code) => ("error", JsValue::from_str(code)),
        };
        field(&object, "kind", &JsValue::from_str(kind));
        if kind != "empty" {
            field(&object, "value", &value);
        }
        object.into()
    }

    /// Sets cell inputs (`rows[i]`, `cols[i]`, `inputs[i]`) and recalculates dependents.
    ///
    /// # Errors
    /// Fails on mismatched lengths, an unknown sheet, an out-of-range cell or too long an input;
    /// nothing is applied then.
    #[wasm_bindgen(js_name = setInputs)]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "`wasm_bindgen` cannot take `&[String]`"
    )]
    pub fn set_inputs(
        &mut self,
        sheet: u32,
        rows: &[u32],
        cols: &[u32],
        inputs: Vec<String>,
    ) -> Result<(), JsError> {
        if rows.len() != cols.len() || rows.len() != inputs.len() {
            return Err(JsError::new("rows, cols and inputs differ in length"));
        }
        let edits: Vec<_> = rows
            .iter()
            .zip(cols)
            .zip(&inputs)
            .map(|((&row, &col), input)| (row, col, input.as_str()))
            .collect();
        self.session
            .set_inputs(sheet as usize, &edits)
            .map_err(|err| JsError::new(&err.to_string()))
    }
}
