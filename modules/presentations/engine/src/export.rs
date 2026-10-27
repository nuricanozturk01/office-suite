use deckcraft_color::Rgba;
use deckcraft_geom::{Size, Xfrm};
use deckcraft_model::{
    Background, ColorRef, Fill, Geom, LayoutType, Line, Presentation, Shape, ShapeId, ShapeKind,
    defaults::{blank_presentation, layout_of_kind, new_slide},
    text::{Align, ParaProps, Paragraph, Run, RunProps, TextBody},
    theme::Theme,
};
use serde_json::Value;

/// Why a deck could not be exported.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// The value does not have the deck shape.
    #[error("content is not a deck")]
    NotADeck,
    /// The engine failed to write the package.
    #[error("could not write the pptx package: {0}")]
    Write(String),
}

/// Renders a deck document as `.pptx` bytes.
///
/// # Errors
/// Fails when `deck` is not a deck or the package cannot be written.
pub fn export_pptx(deck: &Value) -> Result<Vec<u8>, ExportError> {
    let slides = deck
        .get("slides")
        .and_then(Value::as_array)
        .ok_or(ExportError::NotADeck)?;
    let size = Size::new(
        dimension(deck, "width", 960.0),
        dimension(deck, "height", 540.0),
    );
    let mut presentation = blank_presentation(size, Theme::default(), false);
    let layout = layout_of_kind(&presentation, LayoutType::Blank).unwrap_or_default();
    for slide in slides {
        let mut built = new_slide(&mut presentation, layout).ok_or(ExportError::NotADeck)?;
        built.shapes.clear();
        if let Some(color) = slide
            .get("background")
            .and_then(Value::as_str)
            .and_then(Rgba::from_hex)
        {
            built.background = Some(Background::Fill {
                fill: Fill::solid(ColorRef::rgb(color)),
            });
        }
        if let Some(notes) = slide.get("notes").and_then(Value::as_str) {
            built.notes = TextBody::from_text(notes);
        }
        for element in slide
            .get("elements")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let id = presentation.alloc_id().ok_or(ExportError::NotADeck)?;
            built.shapes.push(shape(element, ShapeId(id)));
        }
        presentation.slides.push(std::sync::Arc::new(built));
    }
    write(&presentation)
}

fn write(presentation: &Presentation) -> Result<Vec<u8>, ExportError> {
    deckcraft_pptx::export(presentation).map_err(|err| ExportError::Write(err.to_string()))
}

fn dimension(deck: &Value, key: &str, default: f64) -> f64 {
    deck.get(key)
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite() && (100.0..=10_000.0).contains(v))
        .unwrap_or(default)
}

fn num(element: &Value, key: &str) -> f64 {
    element
        .get(key)
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
        .unwrap_or(0.0)
}

fn color(element: &Value, key: &str) -> Option<Rgba> {
    element
        .get(key)
        .and_then(Value::as_str)
        .and_then(Rgba::from_hex)
}

fn flag(element: &Value, key: &str) -> Option<bool> {
    element.get(key).and_then(Value::as_bool).filter(|v| *v)
}

fn shape(element: &Value, id: ShapeId) -> Shape {
    let is_text = element.get("kind").and_then(Value::as_str) == Some("text");
    let preset = element
        .get("shape")
        .and_then(Value::as_str)
        .unwrap_or("rect");
    let is_line = matches!(preset, "line");
    let geom = Geom::Preset {
        name: match preset {
            "line" => "line",
            "arrow" => "rightArrow",
            other if ["rect", "roundRect", "ellipse", "triangle"].contains(&other) => other,
            _ => "rect",
        }
        .to_owned(),
        adj: vec![],
    };
    let mut built = Shape {
        id,
        name: format!("{} {}", if is_text { "TextBox" } else { "Shape" }, id.0),
        xfrm: Some(Xfrm {
            x: num(element, "x"),
            y: num(element, "y"),
            w: num(element, "w"),
            h: num(element, "h"),
            rot: num(element, "rotation"),
            flip_h: false,
            flip_v: false,
        }),
        kind: ShapeKind::Shape,
        geom,
        text_box: is_text,
        ..Shape::default()
    };
    if !is_text {
        built.fill = Some(match color(element, "fill") {
            Some(c) if !is_line => Fill::solid(ColorRef::rgb(c)),
            _ => Fill::None,
        });
    }
    let stroke = color(element, "stroke");
    let width = element
        .get("strokeWidth")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    built.line = Some(match stroke {
        Some(c) => Line::solid(ColorRef::rgb(c), width),
        None if is_line => Line::solid(ColorRef::rgb(Rgba::rgb(0, 0, 0)), 1.0),
        None => Line::none(),
    });
    if let Some(text) = element.get("text").and_then(Value::as_str) {
        built.text = Some(text_body(element, text));
    }
    built
}

fn text_body(element: &Value, text: &str) -> TextBody {
    let props = RunProps {
        size: element
            .get("fontSize")
            .and_then(Value::as_f64)
            .filter(|s| s.is_finite()),
        bold: flag(element, "bold"),
        italic: flag(element, "italic"),
        underline: flag(element, "underline").map(|_| "sng".to_owned()),
        fill: color(element, "color").map(|c| Fill::solid(ColorRef::rgb(c))),
        ..RunProps::default()
    };
    let align = match element.get("align").and_then(Value::as_str) {
        Some("center") => Some(Align::Center),
        Some("right") => Some(Align::Right),
        _ => None,
    };
    let paragraphs = text
        .split('\n')
        .map(|line| Paragraph {
            props: ParaProps {
                align,
                ..ParaProps::default()
            },
            runs: if line.is_empty() {
                vec![]
            } else {
                vec![Run {
                    text: line.to_owned(),
                    props: props.clone(),
                    ..Run::new("")
                }]
            },
            ..Paragraph::default()
        })
        .collect();
    TextBody {
        paragraphs,
        ..TextBody::default()
    }
}
