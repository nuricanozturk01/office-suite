use deckcraft_model::{
    Fill, Geom, Presentation, Shape, ShapeKind, Slide,
    resolve::{self, Ctx},
    text::Align,
};
use serde_json::{Map, Value, json};

/// Upload size accepted by [`import_pptx`].
pub const MAX_PPTX_BYTES: usize = 30 * 1024 * 1024;

const MAX_SLIDES: usize = 200;
const MAX_ELEMENTS: usize = 500;
const MAX_TEXT_CHARS: usize = 10_000;
const MAX_WARNINGS: usize = 20;

/// A deck read from a file.
#[derive(Debug)]
pub struct Imported {
    /// Deck document in the shape the web slide editor stores.
    pub deck: Value,
    /// What was dropped or approximated, capped at 20 lines.
    pub warnings: Vec<String>,
}

/// Why a file could not be imported.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    /// The upload exceeds [`MAX_PPTX_BYTES`].
    #[error("file is too large")]
    TooLarge,
    /// Not a readable `.pptx` presentation.
    #[error("not a valid pptx file: {0}")]
    Invalid(String),
}

#[derive(Default)]
struct Report(Vec<String>);

impl Report {
    fn note(&mut self, line: &str) {
        if self.0.len() < MAX_WARNINGS && !self.0.iter().any(|l| l == line) {
            self.0.push(line.to_owned());
        }
    }
}

/// Converts a `.pptx` package to a deck document.
///
/// # Errors
/// Fails when the file is too large or not a readable presentation.
pub fn import_pptx(bytes: &[u8]) -> Result<Imported, ImportError> {
    if bytes.len() > MAX_PPTX_BYTES {
        return Err(ImportError::TooLarge);
    }
    let (presentation, skipped) = deckcraft_pptx::import_checked(bytes)
        .map_err(|err| ImportError::Invalid(err.to_string()))?;
    let mut report = Report::default();
    for part in skipped {
        report.note(&format!("Slide {part} could not be read"));
    }
    if presentation.slides.len() > MAX_SLIDES {
        report.note("Slides beyond the 200th were dropped");
    }
    let mut slides: Vec<Value> = presentation
        .slides
        .iter()
        .take(MAX_SLIDES)
        .enumerate()
        .map(|(index, slide)| slide_node(&presentation, slide, index, &mut report))
        .collect();
    if slides.is_empty() {
        slides.push(json!({ "id": "s1", "elements": [] }));
    }
    let size = presentation.slide_size;
    Ok(Imported {
        deck: json!({
            "width": clamp_size(size.width),
            "height": clamp_size(size.height),
            "slides": slides,
        }),
        warnings: report.0,
    })
}

const fn clamp_size(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(100.0, 10_000.0).round()
    } else {
        960.0
    }
}

fn number(value: f64) -> Value {
    let rounded = if value.is_finite() {
        (value * 100.0).round() / 100.0
    } else {
        0.0
    };
    json!(rounded.clamp(-100_000.0, 100_000.0))
}

fn slide_node(
    presentation: &Presentation,
    slide: &Slide,
    index: usize,
    report: &mut Report,
) -> Value {
    let mut node = Map::new();
    node.insert("id".into(), format!("s{}", index + 1).into());
    let Some(ctx) = Ctx::for_slide(presentation, slide) else {
        report.note("A slide without a master was imported empty");
        node.insert("elements".into(), json!([]));
        return Value::Object(node);
    };
    if let (Fill::Solid { color }, _) = resolve::background(&ctx, Some(slide)) {
        node.insert("background".into(), css(ctx.color(&color, None)).into());
    }
    let notes = slide.notes.text();
    if !notes.trim().is_empty() {
        node.insert(
            "notes".into(),
            notes.chars().take(10_000).collect::<String>().into(),
        );
    }
    let mut elements = Vec::new();
    for shape in &slide.shapes {
        if shape.hidden {
            continue;
        }
        if elements.len() >= MAX_ELEMENTS {
            report.note("Shapes beyond 500 on a slide were dropped");
            break;
        }
        match &shape.kind {
            ShapeKind::Shape | ShapeKind::Connector { .. } => {
                elements.push(element_node(&ctx, shape, elements.len()));
            }
            ShapeKind::Picture { .. } => report.note("Pictures were dropped"),
            ShapeKind::Table(_) => report.note("Tables were dropped"),
            ShapeKind::Chart(_) => report.note("Charts were dropped"),
            _ => report.note("Groups, media and other objects were dropped"),
        }
    }
    node.insert("elements".into(), Value::Array(elements));
    Value::Object(node)
}

fn element_node(ctx: &Ctx, shape: &Shape, index: usize) -> Value {
    let xfrm = resolve::xfrm(ctx, shape);
    let (fill, _) = resolve::fill(ctx, shape);
    let fill_hex = match fill {
        Some(Fill::Solid { color }) => Some(css(ctx.color(&color, None))),
        _ => None,
    };
    let text = shape.text.as_ref().map(|body| {
        let all = body.text();
        all.chars().take(MAX_TEXT_CHARS).collect::<String>()
    });
    let has_text = text.as_deref().is_some_and(|t| !t.trim().is_empty());
    let is_line = matches!(shape.kind, ShapeKind::Connector { .. });
    let kind = if has_text && fill_hex.is_none() && !is_line {
        "text"
    } else {
        "shape"
    };

    let mut node = Map::new();
    node.insert("id".into(), format!("e{}", index + 1).into());
    node.insert("kind".into(), kind.into());
    for (key, value) in [("x", xfrm.x), ("y", xfrm.y)] {
        node.insert(key.into(), number(value));
    }
    for (key, value) in [("w", xfrm.w), ("h", xfrm.h)] {
        node.insert(key.into(), number(value.max(0.0)));
    }
    if xfrm.rot.abs() > f64::EPSILON && xfrm.rot.is_finite() {
        node.insert("rotation".into(), number(xfrm.rot.rem_euclid(360.0)));
    }
    if kind == "shape" {
        node.insert("shape".into(), preset(shape, is_line).into());
        if let Some(hex) = fill_hex {
            node.insert("fill".into(), hex.into());
        }
        let (line, _) = resolve::line(ctx, shape);
        if let (Some(Fill::Solid { color }), width) = (line.fill, line.width) {
            node.insert("stroke".into(), css(ctx.color(&color, None)).into());
            node.insert(
                "strokeWidth".into(),
                number(width.unwrap_or(1.0).clamp(0.0, 100.0)),
            );
        }
    }
    if let (Some(body), true) = (shape.text.as_ref(), has_text) {
        node.insert("text".into(), text.unwrap_or_default().into());
        if let Some((paragraph, run)) = body
            .paragraphs
            .iter()
            .find_map(|p| p.runs.iter().find(|r| !r.text.is_empty()).map(|r| (p, r)))
        {
            let props = resolve::run(ctx, shape, paragraph, &run.props);
            if let Some(size) = props.size.filter(|s| s.is_finite()) {
                node.insert("fontSize".into(), number(size.clamp(4.0, 400.0)));
            }
            for (key, flag) in [("bold", props.bold), ("italic", props.italic)] {
                if flag == Some(true) {
                    node.insert(key.into(), true.into());
                }
            }
            node.insert("color".into(), css(resolve::text_color(ctx, &props)).into());
            let align = match resolve::para(ctx, shape, paragraph).align {
                Some(Align::Center) => Some("center"),
                Some(Align::Right) => Some("right"),
                _ => None,
            };
            if let Some(align) = align {
                node.insert("align".into(), align.into());
            }
        }
    }
    Value::Object(node)
}

fn preset(shape: &Shape, is_line: bool) -> &'static str {
    if is_line {
        return "line";
    }
    match &shape.geom {
        Geom::Preset { name, .. } => match name.as_str() {
            "ellipse" => "ellipse",
            "roundRect" => "roundRect",
            "triangle" => "triangle",
            "line" | "straightConnector1" => "line",
            "rightArrow" => "arrow",
            _ => "rect",
        },
        Geom::Custom { .. } => "rect",
    }
}

fn css(color: deckcraft_color::Rgba) -> String {
    format!("#{}", color.hex().to_ascii_lowercase())
}
