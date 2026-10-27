use serde_json::Value;
use wordcraft_doc::{
    Align, Block, Cell, CharProps, ControlWrap, Document, ListKind, Paragraph, Rgb, Row, Run,
    Table, TextColor,
    para::LINE_BREAK,
    para_block,
    props::{CellProps, LineSpacing, NumRef, RowProps, Underline, VertAlign},
};

const TEXT_WIDTH_PT: f32 = 451.0;
const MAX_DEPTH: usize = 24;
const QUOTE_INDENT_PT: f32 = 36.0;
const CODE_FONT: &str = "Courier New";

/// Why a document could not be exported.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// The value is not a `ProseMirror` `doc` node.
    #[error("content is not a document")]
    NotADocument,
    /// The engine failed to write the package.
    #[error("could not write the docx package: {0}")]
    Write(String),
}

/// Renders a `ProseMirror` document (the editor's `getJSON()`) as `.docx` bytes.
///
/// # Errors
/// Fails when `content` is not a `doc` node or the package cannot be written.
pub fn export_docx(content: &Value, title: &str) -> Result<Vec<u8>, ExportError> {
    if content.get("type").and_then(Value::as_str) != Some("doc") {
        return Err(ExportError::NotADocument);
    }
    let mut document = Document::new();
    title.clone_into(&mut document.core.title);
    let mut body = Vec::new();
    for node in children(content) {
        block(&mut document, node, 0, &Context::default(), &mut body);
    }
    if !body.is_empty() {
        document.body = body;
    }
    wordcraft_docx::write(&document).map_err(|err| ExportError::Write(err.to_string()))
}

#[derive(Clone, Copy, Default)]
struct Context {
    list: Option<NumRef>,
    quote: bool,
}

fn children(node: &Value) -> impl Iterator<Item = &Value> {
    node.get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn kind(node: &Value) -> &str {
    node.get("type").and_then(Value::as_str).unwrap_or_default()
}

fn attr<'a>(node: &'a Value, name: &str) -> Option<&'a Value> {
    node.get("attrs").and_then(|attrs| attrs.get(name))
}

fn block(
    document: &mut Document,
    node: &Value,
    depth: usize,
    context: &Context,
    out: &mut Vec<std::sync::Arc<Block>>,
) {
    if depth > MAX_DEPTH {
        return;
    }
    match kind(node) {
        "paragraph" => out.push(para_block(paragraph(node, None, context, false))),
        "heading" => {
            let level = attr(node, "level")
                .and_then(Value::as_u64)
                .map_or(1, |l| l.clamp(1, 4));
            out.push(para_block(paragraph(node, Some(level), context, false)));
        }
        "codeBlock" => out.push(para_block(paragraph(node, None, context, true))),
        "blockquote" => {
            let inner = Context {
                quote: true,
                ..*context
            };
            for child in children(node) {
                block(document, child, depth + 1, &inner, out);
            }
        }
        "bulletList" | "orderedList" => {
            let list_kind = if kind(node) == "bulletList" {
                ListKind::Bullet
            } else {
                ListKind::Numbered
            };
            let num = document.numbering.add_list(list_kind);
            let level = context
                .list
                .map_or(0, |parent| parent.level.saturating_add(1).min(8));
            let inner = Context {
                list: Some(NumRef { num, level }),
                ..*context
            };
            for item in children(node) {
                list_item(document, item, depth + 1, &inner, out);
            }
        }
        "table" => out.push(std::sync::Arc::new(Block::Table(table(
            document, node, depth,
        )))),
        _ => {}
    }
}

fn list_item(
    document: &mut Document,
    item: &Value,
    depth: usize,
    context: &Context,
    out: &mut Vec<std::sync::Arc<Block>>,
) {
    let mut numbered = false;
    for child in children(item) {
        if matches!(kind(child), "paragraph" | "heading") && !numbered {
            numbered = true;
            out.push(para_block(paragraph(child, None, context, false)));
        } else {
            let plain = Context {
                list: if matches!(kind(child), "bulletList" | "orderedList") {
                    context.list
                } else {
                    None
                },
                ..*context
            };
            block(document, child, depth + 1, &plain, out);
        }
    }
}

fn table(document: &mut Document, node: &Value, depth: usize) -> Table {
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    for row in children(node).filter(|n| kind(n) == "tableRow") {
        let mut cells = Vec::new();
        for cell in children(row) {
            let mut blocks = Vec::new();
            for child in children(cell) {
                block(document, child, depth + 1, &Context::default(), &mut blocks);
            }
            if blocks.is_empty() {
                blocks.push(para_block(Paragraph::new()));
            }
            let span = attr(cell, "colspan")
                .and_then(Value::as_u64)
                .map_or(1, |s| u32::try_from(s.clamp(1, 63)).unwrap_or(1));
            cells.push(Cell {
                props: CellProps {
                    span,
                    ..CellProps::default()
                },
                blocks,
                controls: ControlWrap::default(),
            });
        }
        if !cells.is_empty() {
            rows.push(cells);
        }
    }
    let columns = rows
        .iter()
        .map(|cells| cells.iter().map(Cell::span).sum::<usize>())
        .max()
        .unwrap_or(1);
    let mut built = Table::new(rows.len().max(1), columns, TEXT_WIDTH_PT);
    if !rows.is_empty() {
        built.rows = rows
            .into_iter()
            .map(|cells| Row {
                props: RowProps::default(),
                cells,
                controls: ControlWrap::default(),
            })
            .collect();
    }
    built
}

fn paragraph(node: &Value, heading: Option<u64>, context: &Context, code: bool) -> Paragraph {
    let mut built = Paragraph::new();
    let mut runs: Vec<Run> = Vec::new();
    for inline in children(node) {
        let (text, props) = match kind(inline) {
            "text" => (
                inline
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                char_props(inline, code),
            ),
            "hardBreak" => (LINE_BREAK.to_string(), CharProps::default()),
            _ => continue,
        };
        if text.is_empty() {
            continue;
        }
        built.text.push_str(&text);
        match runs.last_mut() {
            Some(last) if last.props == props => last.len += text.len(),
            _ => runs.push(Run {
                len: text.len(),
                props,
            }),
        }
    }
    built.runs = runs;
    if let Some(level) = heading {
        built.props.style = Some(format!("Heading{level}"));
    }
    built.props.align = match attr(node, "textAlign").and_then(Value::as_str) {
        Some("center") => Some(Align::Center),
        Some("right") => Some(Align::Right),
        Some("justify") => Some(Align::Justify),
        _ => None,
    };
    if let Some(multiple) = attr(node, "lineHeight")
        .and_then(Value::as_f64)
        .filter(|v| (0.5..=5.0).contains(v))
    {
        built.props.line_spacing = Some(LineSpacing::Multiple(small(multiple)));
    }
    built.props.style = match attr(node, "docStyle").and_then(Value::as_str) {
        Some("title") => Some("Title".to_owned()),
        Some("subtitle") => Some("Subtitle".to_owned()),
        Some("noSpacing") => Some("NoSpacing".to_owned()),
        Some("quote") => Some("Quote".to_owned()),
        _ => built.props.style.take(),
    };
    for (key, slot) in [
        ("spaceBefore", &mut built.props.space_before),
        ("spaceAfter", &mut built.props.space_after),
        ("indent", &mut built.props.indent_left),
    ] {
        if let Some(points) = attr(node, key)
            .and_then(Value::as_f64)
            .filter(|p| p.is_finite() && (0.0..=1000.0).contains(p))
        {
            *slot = Some(small(points));
        }
    }
    if context.quote {
        built.props.indent_left = Some(QUOTE_INDENT_PT);
    }
    built.props.numbering = context.list;
    built
}

fn char_props(node: &Value, code: bool) -> CharProps {
    let mut props = CharProps::default();
    if code {
        props.font = Some(CODE_FONT.to_owned());
    }
    for mark in children_marks(node) {
        match kind(mark) {
            "bold" => props.bold = Some(true),
            "italic" => props.italic = Some(true),
            "underline" => props.underline = Some(Underline::Single),
            "superscript" => props.vert_align = Some(VertAlign::Superscript),
            "subscript" => props.vert_align = Some(VertAlign::Subscript),
            "strike" => props.strike = Some(true),
            "code" => props.font = Some(CODE_FONT.to_owned()),
            "link" => {
                props.link = attr(mark, "href")
                    .and_then(Value::as_str)
                    .filter(|href| {
                        let lower = href.to_ascii_lowercase();
                        lower.starts_with("https://")
                            || lower.starts_with("http://")
                            || lower.starts_with("mailto:")
                    })
                    .map(str::to_owned);
            }
            "highlight" => {
                props.shading = attr(mark, "color")
                    .and_then(Value::as_str)
                    .and_then(Rgb::parse)
                    .or(Some(Rgb(255, 255, 0)));
            }
            "textStyle" => text_style(mark, &mut props),
            _ => {}
        }
    }
    props
}

fn children_marks(node: &Value) -> impl Iterator<Item = &Value> {
    node.get("marks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn text_style(mark: &Value, props: &mut CharProps) {
    if let Some(color) = attr(mark, "color")
        .and_then(Value::as_str)
        .and_then(Rgb::parse)
    {
        props.color = Some(TextColor::Rgb(color));
    }
    if let Some(family) = attr(mark, "fontFamily").and_then(Value::as_str) {
        let first = family.split(',').next().unwrap_or_default();
        let name = first.trim().trim_matches(|c| c == '"' || c == '\'');
        if !name.is_empty() {
            props.font = Some(name.to_owned());
        }
    }
    if let Some(size) = attr(mark, "fontSize")
        .and_then(Value::as_str)
        .and_then(points)
    {
        props.size = Some(size);
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "callers range-check the value to a few thousand at most"
)]
const fn small(value: f64) -> f32 {
    value as f32
}

fn points(size: &str) -> Option<f32> {
    let size = size.trim();
    let (number, factor) = match size.strip_suffix("pt") {
        Some(number) => (number, 1.0),
        None => (size.strip_suffix("px")?, 0.75),
    };
    let value = number.trim().parse::<f32>().ok()? * factor;
    (1.0..=400.0).contains(&value).then_some(value)
}
