//! Workbook JSON ⇄ `.xlsx` through the vendored engine.

#![expect(
    clippy::unwrap_used,
    reason = "test helpers build fixtures that must not fail"
)]

use std::sync::Arc;

use gridcraft_core::{CellRef, Value};
use gridcraft_model::{
    Cell, Sheet, Workbook,
    style::{NumFmt, StyleId},
};
use serde_json::{Value as Json, json};
use spreadsheets_engine::{ImportError, MAX_XLSX_BYTES, export_xlsx, import_xlsx};

fn sample() -> Json {
    json!({
        "sheets": [
            {
                "id": "s1",
                "name": "Bütçe",
                "cells": {
                    "0,0": { "input": "Kalem", "bold": true, "fill": "#ffeeaa", "align": "center" },
                    "0,1": { "input": "Tutar", "bold": true, "italic": true, "underline": true, "color": "#cc0000" },
                    "1,0": { "input": "Kira" },
                    "1,1": { "input": "1250,5", "format": "currency" },
                    "2,0": { "input": "Zam" },
                    "2,1": { "input": "12%", "format": "percent" },
                    "3,1": { "input": "=TOPLA(B2;B3)*1,5", "format": "number", "align": "right" },
                    "4,0": { "input": "doğru" },
                    "4,1": { "input": "=EĞER(B2>1000;\"yüksek\";\"düşük\")" }
                },
                "cols": { "0": 140, "1": 90 }
            },
            { "id": "s2", "name": "Boş", "cells": {} }
        ]
    })
}

fn cell<'a>(book: &'a Json, sheet: usize, key: &str) -> &'a Json {
    &book["sheets"][sheet]["cells"][key]
}

#[test]
fn workbook_survives_a_roundtrip() {
    let bytes = export_xlsx(&sample()).unwrap();
    assert!(bytes.starts_with(b"PK"), "an xlsx package is a zip file");

    let imported = import_xlsx(&bytes).unwrap();
    assert!(imported.warnings.is_empty(), "{:?}", imported.warnings);
    let book = &imported.workbook;
    assert_eq!(book["sheets"].as_array().unwrap().len(), 2);
    assert_eq!(book["sheets"][0]["name"], "Bütçe");
    assert_eq!(book["sheets"][1]["name"], "Boş");

    let header = cell(book, 0, "0,0");
    assert_eq!(
        (
            header["input"].as_str(),
            header["bold"].as_bool(),
            header["fill"].as_str(),
            header["align"].as_str()
        ),
        (Some("Kalem"), Some(true), Some("#ffeeaa"), Some("center"))
    );
    let amount = cell(book, 0, "0,1");
    assert_eq!(
        (
            amount["italic"].as_bool(),
            amount["underline"].as_bool(),
            amount["color"].as_str()
        ),
        (Some(true), Some(true), Some("#cc0000"))
    );

    // Values and formats: constants come back as plain numbers, formats by name.
    assert_eq!(cell(book, 0, "1,1")["input"], "1250.5");
    assert_eq!(cell(book, 0, "1,1")["format"], "currency");
    assert_eq!(cell(book, 0, "2,1")["input"], "0.12");
    assert_eq!(cell(book, 0, "2,1")["format"], "percent");
    assert_eq!(cell(book, 0, "4,0")["input"], "TRUE");

    // Formulas are normalized to English names and come back in the stored `;` syntax.
    assert_eq!(cell(book, 0, "3,1")["input"], "=SUM(B2;B3)*1,5");
    assert_eq!(cell(book, 0, "3,1")["align"], "right");
    assert_eq!(
        cell(book, 0, "4,1")["input"],
        "=IF(B2>1000;\"yüksek\";\"düşük\")"
    );

    assert_eq!(book["sheets"][0]["cols"]["0"], 140);
    assert_eq!(book["sheets"][0]["cols"]["1"], 90);
}

#[test]
fn exported_formulas_use_the_canonical_file_syntax() {
    let bytes = export_xlsx(&sample()).unwrap();
    let (book, _) = gridcraft_xlsx::read_xlsx(&bytes).unwrap();
    let sheet = book.sheet(0).unwrap();
    let formula = sheet
        .cell(CellRef::new(3, 1))
        .and_then(|c| c.formula.as_ref())
        .unwrap();
    assert_eq!(formula.text, "SUM(B2,B3)*1.5");
}

#[test]
fn garbage_and_oversized_uploads_are_rejected() {
    assert!(matches!(
        import_xlsx(b"definitely not a zip"),
        Err(ImportError::Invalid(_))
    ));
    assert!(matches!(import_xlsx(&[]), Err(ImportError::Invalid(_))));
    let huge = vec![0u8; MAX_XLSX_BYTES + 1];
    assert!(matches!(import_xlsx(&huge), Err(ImportError::TooLarge)));
}

#[test]
fn malformed_workbook_documents_are_rejected_on_export() {
    for bad in [
        json!({}),
        json!({"sheets": []}),
        json!({"sheets": [{"name": "x", "cells": {"a,b": {"input": "1"}}}]}),
    ] {
        assert!(export_xlsx(&bad).is_err(), "{bad}");
    }
}

/// A workbook as another application would write it, built with the engine's own model.
fn foreign_workbook(build: impl FnOnce(&mut Workbook)) -> Vec<u8> {
    let mut book = Workbook::new();
    build(&mut book);
    gridcraft_xlsx::write_xlsx(&book).unwrap()
}

fn sheet_mut(book: &mut Workbook, index: usize) -> &mut Sheet {
    Arc::get_mut(&mut book.sheets[index]).unwrap()
}

#[test]
fn limits_are_enforced_and_reported() {
    let bytes = foreign_workbook(|book| {
        for index in 0..25 {
            if index > 0 {
                book.sheets
                    .push(Arc::new(Sheet::new(format!("Sheet{}", index + 1))));
            }
        }
        let first = sheet_mut(book, 0);
        first.set_cell(CellRef::new(1, 1), Cell::value(Value::number(1.0)));
        first.set_cell(CellRef::new(9_000, 3), Cell::value(Value::number(2.0)));
        first.set_cell(CellRef::new(2, 500), Cell::value(Value::number(3.0)));
    });
    let imported = import_xlsx(&bytes).unwrap();
    let sheets = imported.workbook["sheets"].as_array().unwrap();
    assert_eq!(sheets.len(), 20);
    let cells = sheets[0]["cells"].as_object().unwrap();
    assert_eq!(
        cells.keys().collect::<Vec<_>>(),
        ["1,1"],
        "cells outside the grid are dropped"
    );
    assert!(
        imported
            .warnings
            .iter()
            .any(|w| w.contains("first 20 sheets")),
        "{:?}",
        imported.warnings
    );
    assert!(
        imported.warnings.iter().any(|w| w.contains("beyond row")),
        "{:?}",
        imported.warnings
    );
}

#[test]
fn date_cells_import_as_numbers_with_a_warning() {
    let bytes = foreign_workbook(|book| {
        let style = book.styles.derive(StyleId::default(), |s| {
            s.num_fmt = NumFmt::new("yyyy-mm-dd");
        });
        let sheet = sheet_mut(book, 0);
        sheet.set_cell(
            CellRef::new(0, 0),
            Cell {
                value: Value::number(45000.0),
                formula: None,
                style,
            },
        );
    });
    let imported = import_xlsx(&bytes).unwrap();
    assert_eq!(
        imported.workbook["sheets"][0]["cells"]["0,0"]["input"],
        "45000"
    );
    assert!(
        imported.warnings.iter().any(|w| w.contains("date or time")),
        "{:?}",
        imported.warnings
    );
    assert!(
        imported.workbook["sheets"][0]["cells"]["0,0"]
            .get("format")
            .is_none()
    );
}

#[test]
fn imported_documents_pass_the_workbook_validation_limits() {
    let imported = import_xlsx(&export_xlsx(&sample()).unwrap()).unwrap();
    let text = serde_json::to_string(&imported.workbook).unwrap();
    assert!(text.len() < 1_048_576);
    // Only the keys the workbook schema allows may appear on a cell.
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
    for sheet in imported.workbook["sheets"].as_array().unwrap() {
        for cell in sheet["cells"].as_object().unwrap().values() {
            assert!(
                cell.as_object()
                    .unwrap()
                    .keys()
                    .all(|k| allowed.contains(&k.as_str())),
                "{cell}"
            );
        }
    }
}
