//! Calculation session: the engine behind the web grid.

#![expect(clippy::panic, reason = "test helpers fail loudly")]

use serde_json::{Value, json};
use spreadsheets_engine::{CellValue, EditError, Session};

fn book(cells: &[(&str, &str)]) -> Value {
    let cells: serde_json::Map<String, Value> = cells
        .iter()
        .map(|&(key, input)| (key.to_owned(), json!({ "input": input })))
        .collect();
    json!({ "sheets": [{ "id": "s1", "name": "Sayfa1", "cells": cells }] })
}

fn number(session: &Session, row: u32, col: u32) -> f64 {
    match session.value_at(0, row, col) {
        CellValue::Number(n) => n,
        other => panic!("expected a number at {row},{col}, got {other:?}"),
    }
}

#[test]
fn calculates_on_load_and_after_edits() {
    let mut session = Session::from_json(&book(&[
        ("0,0", "2"),
        ("1,0", "3"),
        ("2,0", "=SUM(A1:A2)*2"),
    ]))
    .unwrap();
    assert!((number(&session, 2, 0) - 10.0).abs() < 1e-9);
    session.set_inputs(0, &[(0, 0, "10")]).unwrap();
    assert!((number(&session, 2, 0) - 26.0).abs() < 1e-9);
    session.set_inputs(0, &[(0, 0, "")]).unwrap();
    assert_eq!(session.value_at(0, 0, 0), CellValue::Empty);
    assert!((number(&session, 2, 0) - 6.0).abs() < 1e-9);
}

#[test]
fn understands_the_turkish_stored_syntax() {
    let session = Session::from_json(&book(&[
        ("0,0", "1250,5"),
        ("1,0", "=TOPLA(A1;2,5)"),
        ("2,0", "=EĞER(A1>1000;\"yüksek\";\"düşük\")"),
    ]))
    .unwrap();
    assert!((number(&session, 1, 0) - 1253.0).abs() < 1e-9);
    assert_eq!(
        session.value_at(0, 2, 0),
        CellValue::Text("yüksek".to_owned())
    );
}

#[test]
fn covers_functions_the_old_grid_lacked() {
    let session = Session::from_json(&book(&[
        ("0,0", "a"),
        ("0,1", "1"),
        ("1,0", "b"),
        ("1,1", "2"),
        ("2,0", "=XLOOKUP(\"b\";A1:A2;B1:B2)"),
        ("3,0", "=SUMIFS(B1:B2;A1:A2;\"a\")"),
        ("4,0", "=TEXTJOIN(\"-\";TRUE;A1:A2)"),
        ("5,0", "=ROUND(PMT(0,01;12;-1000);2)"),
    ]))
    .unwrap();
    assert!((number(&session, 2, 0) - 2.0).abs() < 1e-9);
    assert!((number(&session, 3, 0) - 1.0).abs() < 1e-9);
    assert_eq!(session.value_at(0, 4, 0), CellValue::Text("a-b".to_owned()));
    assert!((number(&session, 5, 0) - 88.85).abs() < 1e-9);
}

#[test]
fn dynamic_arrays_spill_into_neighbours() {
    let session = Session::from_json(&book(&[("0,0", "=SEQUENCE(3)")])).unwrap();
    assert!((number(&session, 0, 0) - 1.0).abs() < 1e-9);
    assert!((number(&session, 1, 0) - 2.0).abs() < 1e-9);
    assert!((number(&session, 2, 0) - 3.0).abs() < 1e-9);
}

#[test]
fn reports_errors_and_cycles() {
    let session = Session::from_json(&book(&[
        ("0,0", "=1/0"),
        ("1,0", "=NOSUCHFN(1)"),
        ("2,0", "=C3"),
        ("2,2", "=A3"),
    ]))
    .unwrap();
    assert_eq!(session.value_at(0, 0, 0), CellValue::Error("#DIV/0!"));
    assert_eq!(session.value_at(0, 1, 0), CellValue::Error("#NAME?"));
    assert!(matches!(session.value_at(0, 2, 0), CellValue::Error(_)));
}

#[test]
fn reads_across_sheets() {
    let doc = json!({ "sheets": [
        { "id": "s1", "name": "Girdi", "cells": { "0,0": { "input": "21" } } },
        { "id": "s2", "name": "Toplam", "cells": { "0,0": { "input": "=Girdi!A1*2" } } },
    ]});
    let mut session = Session::from_json(&doc).unwrap();
    assert!(
        (match session.value_at(1, 0, 0) {
            CellValue::Number(n) => n,
            other => panic!("{other:?}"),
        } - 42.0)
            .abs()
            < 1e-9
    );
    session.set_inputs(0, &[(0, 0, "5")]).unwrap();
    assert_eq!(session.value_at(1, 0, 0), CellValue::Number(10.0));
}

#[test]
fn refuses_bad_edits_without_applying_any() {
    let mut session = Session::from_json(&book(&[("0,0", "1")])).unwrap();
    assert_eq!(
        session.set_inputs(3, &[(0, 0, "2")]),
        Err(EditError::NoSuchSheet)
    );
    assert_eq!(
        session.set_inputs(0, &[(0, 0, "2"), (9_999, 0, "x")]),
        Err(EditError::OutOfBounds)
    );
    let long = "x".repeat(8_001);
    assert_eq!(
        session.set_inputs(0, &[(0, 0, &long)]),
        Err(EditError::TooLong)
    );
    assert_eq!(session.value_at(0, 0, 0), CellValue::Number(1.0));
}

#[test]
fn rejects_malformed_documents() {
    assert!(Session::from_json(&json!({ "sheets": [] })).is_err());
    assert!(Session::from_json(&json!({ "nope": 1 })).is_err());
}

#[test]
fn bare_decimal_comma_formula() {
    let session = Session::from_json(&book(&[("0,0", "=1,5+2")])).unwrap();
    assert_eq!(session.value_at(0, 0, 0), CellValue::Number(3.5));
}

#[test]
fn unparsable_formula_reads_as_name_error() {
    let session = Session::from_json(&book(&[("0,0", "=1+*")])).unwrap();
    assert_eq!(session.value_at(0, 0, 0), CellValue::Error("#NAME?"));
}
