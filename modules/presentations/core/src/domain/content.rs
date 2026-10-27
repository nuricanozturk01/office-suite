//! Slide deck stored as JSON. The shape is the contract with the web slide editor.

use std::collections::HashSet;

use serde_json::{Map, Value};
use thiserror::Error;

const MAX_BYTES: usize = 4_194_304;
const MAX_SLIDES: usize = 200;
const MAX_ELEMENTS: usize = 500;
const MAX_TEXT: usize = 10_000;
const MAX_NOTES: usize = 10_000;
const MAX_COORD: f64 = 100_000.0;

const KINDS: &[&str] = &["text", "shape"];
const SHAPES: &[&str] = &["rect", "roundRect", "ellipse", "triangle", "line", "arrow"];
const ALIGNS: &[&str] = &["left", "center", "right"];
const FLAGS: &[&str] = &["bold", "italic", "underline"];
const ELEMENT_KEYS: &[&str] = &[
    "id",
    "kind",
    "x",
    "y",
    "w",
    "h",
    "rotation",
    "text",
    "fontSize",
    "bold",
    "italic",
    "underline",
    "color",
    "fill",
    "stroke",
    "strokeWidth",
    "align",
    "shape",
];

/// Rejected deck document.
#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum ContentError {
    /// Shape does not match the deck schema.
    #[error("deck is invalid")]
    Invalid,
    /// Serialized document exceeds the size cap.
    #[error("deck is too large")]
    TooLarge,
}

/// Validated deck JSON.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DeckContent(Value);

impl DeckContent {
    /// Deck with one blank slide.
    #[must_use]
    pub(crate) fn empty(slide_id: &str) -> Self {
        Self(serde_json::json!({
            "width": 960,
            "height": 540,
            "slides": [{ "id": slide_id, "elements": [
                { "id": format!("{slide_id}-title"), "kind": "text", "x": 80, "y": 170,
                  "w": 800, "h": 100, "text": "", "fontSize": 48, "bold": true,
                  "align": "center", "color": "#202124" },
                { "id": format!("{slide_id}-subtitle"), "kind": "text", "x": 160, "y": 290,
                  "w": 640, "h": 70, "text": "", "fontSize": 24, "align": "center",
                  "color": "#5f6368" }
            ] }]
        }))
    }

    /// Parses a client-supplied deck.
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
    if root
        .keys()
        .any(|k| !["width", "height", "slides"].contains(&k.as_str()))
    {
        return Err(ContentError::Invalid);
    }
    for key in ["width", "height"] {
        if !number_in(root.get(key), 100.0, 10_000.0) {
            return Err(ContentError::Invalid);
        }
    }
    let slides = root
        .get("slides")
        .and_then(Value::as_array)
        .ok_or(ContentError::Invalid)?;
    if slides.is_empty() || slides.len() > MAX_SLIDES {
        return Err(ContentError::Invalid);
    }
    let mut ids = HashSet::new();
    for slide in slides {
        validate_slide(slide, &mut ids)?;
    }
    Ok(())
}

fn validate_slide(value: &Value, ids: &mut HashSet<String>) -> Result<(), ContentError> {
    let slide = value.as_object().ok_or(ContentError::Invalid)?;
    if slide
        .keys()
        .any(|k| !["id", "background", "notes", "elements"].contains(&k.as_str()))
    {
        return Err(ContentError::Invalid);
    }
    valid_id(slide, ids)?;
    if let Some(background) = slide.get("background")
        && !background.as_str().is_some_and(is_hex_color)
    {
        return Err(ContentError::Invalid);
    }
    if let Some(notes) = slide.get("notes")
        && notes.as_str().is_none_or(|n| n.chars().count() > MAX_NOTES)
    {
        return Err(ContentError::Invalid);
    }
    let elements = slide
        .get("elements")
        .and_then(Value::as_array)
        .ok_or(ContentError::Invalid)?;
    if elements.len() > MAX_ELEMENTS {
        return Err(ContentError::TooLarge);
    }
    let mut element_ids = HashSet::new();
    for element in elements {
        validate_element(element, &mut element_ids)?;
    }
    Ok(())
}

fn validate_element(value: &Value, ids: &mut HashSet<String>) -> Result<(), ContentError> {
    let element = value.as_object().ok_or(ContentError::Invalid)?;
    if element.keys().any(|k| !ELEMENT_KEYS.contains(&k.as_str())) {
        return Err(ContentError::Invalid);
    }
    valid_id(element, ids)?;
    let kind = element.get("kind").and_then(Value::as_str);
    if !kind.is_some_and(|k| KINDS.contains(&k)) {
        return Err(ContentError::Invalid);
    }
    for key in ["x", "y"] {
        if !number_in(element.get(key), -MAX_COORD, MAX_COORD) {
            return Err(ContentError::Invalid);
        }
    }
    for key in ["w", "h"] {
        if !number_in(element.get(key), 0.0, MAX_COORD) {
            return Err(ContentError::Invalid);
        }
    }
    optional(element, "rotation", |v| number_in(Some(v), -360.0, 360.0))?;
    optional(element, "fontSize", |v| number_in(Some(v), 4.0, 400.0))?;
    optional(element, "strokeWidth", |v| number_in(Some(v), 0.0, 100.0))?;
    optional(element, "text", |v| {
        v.as_str().is_some_and(|t| t.chars().count() <= MAX_TEXT)
    })?;
    optional(element, "align", |v| {
        v.as_str().is_some_and(|a| ALIGNS.contains(&a))
    })?;
    optional(element, "shape", |v| {
        v.as_str().is_some_and(|s| SHAPES.contains(&s))
    })?;
    for key in ["color", "fill", "stroke"] {
        optional(element, key, |v| v.as_str().is_some_and(is_hex_color))?;
    }
    for key in FLAGS {
        optional(element, key, Value::is_boolean)?;
    }
    Ok(())
}

fn optional(
    object: &Map<String, Value>,
    key: &str,
    check: impl Fn(&Value) -> bool,
) -> Result<(), ContentError> {
    match object.get(key) {
        Some(value) if !check(value) => Err(ContentError::Invalid),
        _ => Ok(()),
    }
}

fn number_in(value: Option<&Value>, min: f64, max: f64) -> bool {
    value
        .and_then(Value::as_f64)
        .is_some_and(|n| n.is_finite() && (min..=max).contains(&n))
}

fn valid_id(object: &Map<String, Value>, ids: &mut HashSet<String>) -> Result<(), ContentError> {
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or(ContentError::Invalid)?;
    if id.is_empty() || id.chars().count() > 64 || !ids.insert(id.to_owned()) {
        return Err(ContentError::Invalid);
    }
    Ok(())
}

fn is_hex_color(value: &str) -> bool {
    value
        .strip_prefix('#')
        .is_some_and(|hex| hex.len() == 6 && hex.chars().all(|char| char.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deck() -> Value {
        serde_json::json!({
            "width": 960, "height": 540,
            "slides": [{ "id": "s1", "background": "#ffffff", "elements": [
                { "id": "e1", "kind": "text", "x": 10, "y": 20, "w": 300, "h": 80,
                  "text": "Merhaba", "fontSize": 32, "bold": true, "color": "#112233" },
                { "id": "e2", "kind": "shape", "shape": "ellipse", "x": 0, "y": 0, "w": 50, "h": 50,
                  "fill": "#ff0000" }
            ] }]
        })
    }

    #[test]
    fn accepts_a_deck() {
        assert!(DeckContent::parse(deck()).is_ok());
    }

    #[test]
    fn rejects_bad_colors_unknown_keys_and_duplicate_ids() {
        let mut bad = deck();
        bad["slides"][0]["elements"][0]["color"] = Value::String("red".to_owned());
        assert_eq!(DeckContent::parse(bad), Err(ContentError::Invalid));
        let mut bad = deck();
        bad["slides"][0]["elements"][0]["onclick"] = Value::Bool(true);
        assert_eq!(DeckContent::parse(bad), Err(ContentError::Invalid));
        let mut bad = deck();
        bad["slides"][0]["elements"][1]["id"] = Value::String("e1".to_owned());
        assert_eq!(DeckContent::parse(bad), Err(ContentError::Invalid));
    }

    #[test]
    fn rejects_an_empty_slide_list() {
        let mut bad = deck();
        bad["slides"] = serde_json::json!([]);
        assert_eq!(DeckContent::parse(bad), Err(ContentError::Invalid));
    }
}
