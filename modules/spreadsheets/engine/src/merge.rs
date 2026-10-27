//! Three-way merge of `.xlsx` files: two people edited one workbook concurrently.
//!
//! Cells are the unit: changes to different cells are all kept, different changes to one cell
//! are a conflict. A sheet's other settings (column widths, merges, freeze panes) and the
//! workbook's styles are taken from whichever side changed them; if both did, differently, the
//! save conflicts.

use std::{collections::BTreeSet, sync::Arc};

use gridcraft_model::{CellStore, Sheet};
use storage_contracts::{MergeError, PackageMerger};

/// Merges `.xlsx` files; see the module docs.
#[derive(Debug, Default, Clone, Copy)]
pub struct XlsxMerger;

/// The side that changed `base`, or either when both made the same change.
fn pick<T: PartialEq>(base: &T, ours: T, theirs: T) -> Result<T, MergeError> {
    if ours == theirs || theirs == *base {
        Ok(ours)
    } else if ours == *base {
        Ok(theirs)
    } else {
        Err(MergeError::Conflict)
    }
}

fn without_cells(sheet: &Sheet) -> Sheet {
    let mut rest = sheet.clone();
    rest.cells = CellStore::new();
    rest
}

fn merge_sheet(base: &Sheet, ours: &Sheet, theirs: &Sheet) -> Result<Sheet, MergeError> {
    let mut merged = pick(
        &without_cells(base),
        without_cells(ours),
        without_cells(theirs),
    )?;
    let positions: BTreeSet<_> = [base, ours, theirs]
        .into_iter()
        .flat_map(|sheet| sheet.cells.iter().map(|(at, _)| at))
        .collect();
    for at in positions {
        let cell = pick(
            &base.cells.get(at),
            ours.cells.get(at),
            theirs.cells.get(at),
        )?;
        if let Some(cell) = cell {
            merged.cells.set(at, cell.clone());
        }
    }
    Ok(merged)
}

impl PackageMerger for XlsxMerger {
    fn handles(&self, name: &str) -> bool {
        name.rsplit_once('.')
            .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("xlsx"))
    }

    fn merge(&self, base: &[u8], ours: &[u8], theirs: &[u8]) -> Result<Vec<u8>, MergeError> {
        let read = |bytes: &[u8]| {
            gridcraft_xlsx::read_xlsx(bytes)
                .map(|(book, _)| book)
                .map_err(|_| MergeError::Unreadable)
        };
        let (base, ours, theirs) = (read(base)?, read(ours)?, read(theirs)?);
        let names = |book: &gridcraft_model::Workbook| -> Vec<String> {
            book.sheets.iter().map(|s| s.name.clone()).collect()
        };
        let order = merge3::merge3(&names(&base), &names(&ours), &names(&theirs))
            .map_err(|_| MergeError::Conflict)?;
        let find = |book: &gridcraft_model::Workbook, name: &str| {
            book.sheets.iter().find(|s| s.name == name).cloned()
        };
        let mut sheets = Vec::with_capacity(order.len());
        for name in &order {
            let (b, o, t) = (find(&base, name), find(&ours, name), find(&theirs, name));
            let sheet = match (b, o, t) {
                (Some(b), Some(o), Some(t)) => merge_sheet(&b, &o, &t)?,
                // Added by one side (or by both identically).
                (None, Some(o), None) => o.as_ref().clone(),
                (None, None, Some(t)) => t.as_ref().clone(),
                (None, Some(o), Some(t)) if o == t => o.as_ref().clone(),
                _ => return Err(MergeError::Conflict),
            };
            sheets.push(Arc::new(sheet));
        }
        let mut merged = pick_book(&base, &ours, &theirs)?;
        merged.sheets = sheets;
        gridcraft_xlsx::write_xlsx(&merged).map_err(|_| MergeError::Unreadable)
    }
}

/// Workbook-level settings (styles, names, calculation) of the side that changed them.
fn pick_book(
    base: &gridcraft_model::Workbook,
    ours: &gridcraft_model::Workbook,
    theirs: &gridcraft_model::Workbook,
) -> Result<gridcraft_model::Workbook, MergeError> {
    let without_sheets = |book: &gridcraft_model::Workbook| {
        let mut rest = book.clone();
        rest.sheets = Vec::new();
        rest
    };
    pick(
        &without_sheets(base),
        without_sheets(ours),
        without_sheets(theirs),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gridcraft_core::{CellRef, Value};
    use gridcraft_model::{Cell, Workbook};

    fn book(cells: &[(u32, u32, f64)]) -> Vec<u8> {
        let mut book = Workbook::default();
        let mut sheet = Sheet::new("Sheet1");
        for &(row, col, n) in cells {
            sheet
                .cells
                .set(CellRef::new(row, col), Cell::value(Value::Number(n)));
        }
        book.sheets = vec![Arc::new(sheet)];
        gridcraft_xlsx::write_xlsx(&book).unwrap()
    }

    fn number(bytes: &[u8], row: u32, col: u32) -> Option<f64> {
        let (book, _) = gridcraft_xlsx::read_xlsx(bytes).unwrap();
        match book.sheets[0].cells.get(CellRef::new(row, col))?.value {
            Value::Number(n) => Some(n),
            _ => None,
        }
    }

    #[test]
    fn changes_to_different_cells_are_both_kept() {
        let base = book(&[(0, 0, 1.0), (1, 0, 2.0)]);
        let ours = book(&[(0, 0, 10.0), (1, 0, 2.0)]);
        let theirs = book(&[(0, 0, 1.0), (1, 0, 2.0), (2, 0, 3.0)]);
        let merged = XlsxMerger.merge(&base, &ours, &theirs).unwrap();
        assert_eq!(number(&merged, 0, 0), Some(10.0));
        assert_eq!(number(&merged, 1, 0), Some(2.0));
        assert_eq!(number(&merged, 2, 0), Some(3.0));
    }

    #[test]
    fn a_cell_deleted_by_one_side_stays_deleted() {
        let base = book(&[(0, 0, 1.0), (1, 0, 2.0)]);
        let ours = book(&[(0, 0, 1.0)]);
        let theirs = book(&[(0, 0, 5.0), (1, 0, 2.0)]);
        let merged = XlsxMerger.merge(&base, &ours, &theirs).unwrap();
        assert_eq!(number(&merged, 0, 0), Some(5.0));
        assert_eq!(number(&merged, 1, 0), None);
    }

    #[test]
    fn one_cell_changed_two_ways_conflicts() {
        let base = book(&[(0, 0, 1.0)]);
        assert_eq!(
            XlsxMerger.merge(&base, &book(&[(0, 0, 2.0)]), &book(&[(0, 0, 3.0)])),
            Err(MergeError::Conflict)
        );
    }

    #[test]
    fn garbage_is_unreadable() {
        let base = book(&[]);
        assert_eq!(
            XlsxMerger.merge(&base, b"nope", &base),
            Err(MergeError::Unreadable)
        );
    }
}
