//! Conversion between the formula text stored in a workbook and the canonical file syntax.
//!
//! Stored formulas are what the web grid accepts: English or Turkish function names and `;`
//! between arguments with `,` as the decimal mark (`=TOPLA(A1;B1;1,5)`). Files use English
//! names, `,` between arguments and `.` as the decimal mark.

/// Turkish function names the grid understands, with their canonical names.
const TURKISH_NAMES: &[(&str, &str)] = &[
    ("TOPLA", "SUM"),
    ("ORTALAMA", "AVERAGE"),
    ("MAK", "MAX"),
    ("EGER", "IF"),
    ("VE", "AND"),
    ("YADA", "OR"),
    ("DEGIL", "NOT"),
    ("YUVARLA", "ROUND"),
    ("MUTLAK", "ABS"),
    ("BIRLESTIR", "CONCAT"),
    ("UZUNLUK", "LEN"),
    ("TAMSAYI", "INT"),
];

/// Where a scanner is inside a formula.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Zone {
    Code,
    Text,
    SheetName,
}

fn fold(name: &str) -> String {
    name.chars()
        .flat_map(char::to_uppercase)
        .map(|c| match c {
            'İ' => 'I',
            'Ş' => 'S',
            'Ğ' => 'G',
            'Ü' => 'U',
            'Ö' => 'O',
            'Ç' => 'C',
            other => other,
        })
        .collect()
}

/// Stored formula body (without `=`) to canonical file syntax.
#[must_use]
pub(crate) fn to_canonical(stored: &str) -> String {
    let semicolons = has_code_char(stored, ';');
    let mut out = String::with_capacity(stored.len());
    let mut zone = Zone::Code;
    let mut brackets = 0usize;
    let mut parens = 0usize;
    let mut braces = 0usize;
    let chars: Vec<char> = stored.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match zone {
            Zone::Text => {
                out.push(c);
                if c == '"' {
                    zone = Zone::Code;
                }
            }
            Zone::SheetName => {
                out.push(c);
                if c == '\'' {
                    zone = Zone::Code;
                }
            }
            Zone::Code => match c {
                '"' => {
                    zone = Zone::Text;
                    out.push(c);
                }
                '\'' => {
                    zone = Zone::SheetName;
                    out.push(c);
                }
                '[' => {
                    brackets += 1;
                    out.push(c);
                }
                ']' => {
                    brackets = brackets.saturating_sub(1);
                    out.push(c);
                }
                '(' => {
                    parens += 1;
                    out.push(c);
                }
                ')' => {
                    parens = parens.saturating_sub(1);
                    out.push(c);
                }
                '{' => {
                    braces += 1;
                    out.push(c);
                }
                '}' => {
                    braces = braces.saturating_sub(1);
                    out.push(c);
                }
                ';' if brackets == 0 => out.push(','),
                // Outside parentheses a comma cannot separate arguments, so `=1,5+2` is 3.5.
                ',' if (semicolons || parens == 0)
                    && brackets == 0
                    && braces == 0
                    && is_decimal_comma(&chars, i) =>
                {
                    out.push('.');
                }
                c if c.is_alphabetic() || c == '_' => {
                    let start = i;
                    while i + 1 < chars.len() && is_name_char(chars[i + 1]) {
                        i += 1;
                    }
                    let word: String = chars[start..=i].iter().collect();
                    let is_call = chars.get(i + 1) == Some(&'(');
                    out.push_str(&if is_call {
                        canonical_function(&word)
                    } else {
                        word
                    });
                }
                other => out.push(other),
            },
        }
        i += 1;
    }
    out
}

#[cfg(any(feature = "xlsx", test))]
/// Canonical file syntax body to the stored syntax (`;` separators, `,` decimals).
#[must_use]
pub(crate) fn to_stored(canonical: &str) -> String {
    let mut out = String::with_capacity(canonical.len());
    let mut zone = Zone::Code;
    let mut brackets = 0usize;
    let chars: Vec<char> = canonical.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        match zone {
            Zone::Text => {
                out.push(c);
                if c == '"' {
                    zone = Zone::Code;
                }
            }
            Zone::SheetName => {
                out.push(c);
                if c == '\'' {
                    zone = Zone::Code;
                }
            }
            Zone::Code => match c {
                '"' => {
                    zone = Zone::Text;
                    out.push(c);
                }
                '\'' => {
                    zone = Zone::SheetName;
                    out.push(c);
                }
                '[' => {
                    brackets += 1;
                    out.push(c);
                }
                ']' => {
                    brackets = brackets.saturating_sub(1);
                    out.push(c);
                }
                ',' if brackets == 0 => out.push(';'),
                '.' if brackets == 0 && is_decimal_point(&chars, i) => out.push(','),
                other => out.push(other),
            },
        }
    }
    out
}

fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.'
}

fn canonical_function(word: &str) -> String {
    let folded = fold(word);
    TURKISH_NAMES
        .iter()
        .find(|(turkish, _)| *turkish == folded)
        .map_or_else(|| word.to_owned(), |(_, english)| (*english).to_owned())
}

fn has_code_char(text: &str, wanted: char) -> bool {
    let mut quoted = false;
    let mut sheet = false;
    for c in text.chars() {
        match c {
            '"' if !sheet => quoted = !quoted,
            '\'' if !quoted => sheet = !sheet,
            c if c == wanted && !quoted && !sheet => return true,
            _ => {}
        }
    }
    false
}

fn digit_at(chars: &[char], index: Option<usize>) -> bool {
    index
        .and_then(|i| chars.get(i))
        .is_some_and(char::is_ascii_digit)
}

/// `1,5` in a `;`-separated formula: a comma between two digits.
fn is_decimal_comma(chars: &[char], i: usize) -> bool {
    digit_at(chars, i.checked_sub(1)) && digit_at(chars, Some(i + 1))
}

#[cfg(any(feature = "xlsx", test))]
/// `1.5` in canonical syntax: a point between two digits that is not part of a name or a
/// reference like `Sheet1.A1`.
fn is_decimal_point(chars: &[char], i: usize) -> bool {
    if !digit_at(chars, i.checked_sub(1)) || !digit_at(chars, Some(i + 1)) {
        return false;
    }
    // Walk back over the digits: the number must not be glued to a letter (e.g. `A1.5`).
    let mut j = i;
    while j > 0 && chars[j - 1].is_ascii_digit() {
        j -= 1;
    }
    !(j > 0 && (chars[j - 1].is_alphabetic() || chars[j - 1] == '_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turkish_input_becomes_canonical() {
        assert_eq!(to_canonical("TOPLA(A1:A3;B1)"), "SUM(A1:A3,B1)");
        assert_eq!(
            to_canonical("EĞER(A1>1,5;\"a;b\";0)"),
            "IF(A1>1.5,\"a;b\",0)"
        );
        assert_eq!(
            to_canonical("eger(A1>1,5;\"a;b\";0)"),
            "IF(A1>1.5,\"a;b\",0)"
        );
    }

    #[test]
    fn english_input_with_commas_is_left_alone() {
        assert_eq!(to_canonical("SUM(A1,B1)*2"), "SUM(A1,B1)*2");
        assert_eq!(to_canonical("ROUND(A1,2)"), "ROUND(A1,2)");
    }

    #[test]
    fn quoted_text_and_sheet_names_are_untouched() {
        assert_eq!(
            to_canonical("CONCAT(\"a;b,1\";'My;Sheet'!A1)"),
            "CONCAT(\"a;b,1\",'My;Sheet'!A1)"
        );
        assert_eq!(
            to_canonical("A1*0,5"),
            "A1*0.5",
            "outside parentheses a comma between digits is a decimal comma"
        );
        assert_eq!(
            to_canonical("SUM(1,5)"),
            "SUM(1,5)",
            "inside parentheses and without a semicolon it separates arguments"
        );
    }

    #[test]
    fn canonical_becomes_stored() {
        assert_eq!(to_stored("SUM(A1,B1,1.5)"), "SUM(A1;B1;1,5)");
        assert_eq!(
            to_stored("IF(A1>0.25,\"x,y\",'S,1'!B2)"),
            "IF(A1>0,25;\"x,y\";'S,1'!B2)"
        );
        assert_eq!(
            to_stored("Table1[[#This Row],[Col]]"),
            "Table1[[#This Row],[Col]]"
        );
    }

    #[test]
    fn stored_and_canonical_roundtrip() {
        for canonical in ["SUM(A1:A3,B1)", "IF(A1>1.5,\"a,b\",0)", "ROUND(A1*0.07,2)"] {
            assert_eq!(to_canonical(&to_stored(canonical)), canonical);
        }
    }
}
