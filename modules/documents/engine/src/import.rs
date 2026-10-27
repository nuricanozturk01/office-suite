use serde_json::{Map, Value, json};
use wordcraft_doc::{
    Align, Block, CharProps, Document, Paragraph, Table, TextColor,
    para::{LINE_BREAK, OBJ},
    props::{LineSpacing, Underline, VMerge, VertAlign},
    section::NumFormat,
};

/// Upload size accepted by [`import_docx`].
pub const MAX_DOCX_BYTES: usize = 20 * 1024 * 1024;

const MAX_TABLE_DEPTH: usize = 6;
const MAX_WARNINGS: usize = 20;

/// A document read from a file.
#[derive(Debug)]
pub struct Imported {
    /// `ProseMirror` document (`{ "type": "doc", ... }`) in the editor schema.
    pub content: Value,
    /// What was dropped or approximated, capped at 20 lines.
    pub warnings: Vec<String>,
}

/// Why a file could not be imported.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    /// The upload exceeds [`MAX_DOCX_BYTES`].
    #[error("file is too large")]
    TooLarge,
    /// Not a readable `.docx` document.
    #[error("not a valid docx file: {0}")]
    Invalid(String),
}

#[derive(Default)]
struct Report {
    lines: Vec<String>,
}

impl Report {
    fn note(&mut self, line: &str) {
        if self.lines.len() < MAX_WARNINGS && !self.lines.iter().any(|l| l == line) {
            self.lines.push(line.to_owned());
        }
    }
}

struct ListItem {
    level: u8,
    ordered: bool,
    paragraph: Value,
}

struct OpenList {
    level: u8,
    ordered: bool,
    items: Vec<Value>,
}

/// Converts a `.docx` package to a `ProseMirror` document.
///
/// # Errors
/// Fails when the file is too large or not a readable Word document.
pub fn import_docx(bytes: &[u8]) -> Result<Imported, ImportError> {
    if bytes.len() > MAX_DOCX_BYTES {
        return Err(ImportError::TooLarge);
    }
    let document =
        wordcraft_docx::read(bytes).map_err(|err| ImportError::Invalid(err.to_string()))?;
    let mut report = Report::default();
    let mut blocks = convert_blocks(
        &document,
        document.body.iter().map(|b| &**b),
        0,
        &mut report,
    );
    if blocks.is_empty() {
        blocks.push(json!({ "type": "paragraph" }));
    }
    Ok(Imported {
        content: json!({ "type": "doc", "content": blocks }),
        warnings: report.lines,
    })
}

fn convert_blocks<'a>(
    document: &Document,
    blocks: impl Iterator<Item = &'a Block>,
    depth: usize,
    report: &mut Report,
) -> Vec<Value> {
    let mut out = Vec::new();
    let mut pending: Vec<ListItem> = Vec::new();
    for block in blocks {
        match block {
            Block::Para(paragraph) => {
                if let Some((level, ordered)) = list_info(document, paragraph) {
                    pending.push(ListItem {
                        level,
                        ordered,
                        paragraph: paragraph_node(document, paragraph, "paragraph", None, report),
                    });
                    continue;
                }
                flush_lists(&mut pending, &mut out);
                out.push(match heading_level(document, paragraph) {
                    Some(level) => {
                        paragraph_node(document, paragraph, "heading", Some(level), report)
                    }
                    None => paragraph_node(document, paragraph, "paragraph", None, report),
                });
            }
            Block::Table(table) => {
                flush_lists(&mut pending, &mut out);
                if depth >= MAX_TABLE_DEPTH {
                    report.note("Deeply nested tables were dropped");
                } else {
                    out.push(table_node(document, table, depth, report));
                }
            }
        }
    }
    flush_lists(&mut pending, &mut out);
    out
}

fn list_info(document: &Document, paragraph: &Paragraph) -> Option<(u8, bool)> {
    let reference = paragraph.props.numbering?;
    if reference.num == 0 {
        return None;
    }
    let ordered = document
        .numbering
        .level(reference.num, reference.level)
        .is_some_and(|level| level.format != NumFormat::Bullet);
    Some((reference.level, ordered))
}

/// The named editor style (`docStyle`) a Word paragraph style maps to.
fn doc_style(document: &Document, paragraph: &Paragraph) -> Option<&'static str> {
    let style = paragraph.props.style.as_deref()?;
    let name = document
        .styles
        .get(style)
        .map_or(style, |found| found.name.as_str())
        .to_ascii_lowercase();
    match name.as_str() {
        "title" => Some("title"),
        "subtitle" => Some("subtitle"),
        "no spacing" => Some("noSpacing"),
        "quote" | "intense quote" => Some("quote"),
        _ => None,
    }
}

fn heading_level(document: &Document, paragraph: &Paragraph) -> Option<u8> {
    if let Some(style) = paragraph.props.style.as_deref() {
        let name = document
            .styles
            .get(style)
            .map_or(style, |found| found.name.as_str())
            .to_ascii_lowercase();
        if let Some(digit) = name
            .strip_prefix("heading ")
            .and_then(|n| n.parse::<u8>().ok())
        {
            return Some(digit.clamp(1, 4));
        }
    }
    paragraph
        .props
        .outline_level
        .filter(|l| *l < 4)
        .map(|l| l + 1)
}

fn flush_lists(pending: &mut Vec<ListItem>, out: &mut Vec<Value>) {
    let mut stack: Vec<OpenList> = Vec::new();
    for item in pending.drain(..) {
        while stack.last().is_some_and(|open| open.level > item.level) {
            close_list(&mut stack, out);
        }
        if stack
            .last()
            .is_some_and(|open| open.level == item.level && open.ordered != item.ordered)
        {
            close_list(&mut stack, out);
        }
        if stack.last().is_none_or(|open| open.level < item.level) {
            stack.push(OpenList {
                level: item.level,
                ordered: item.ordered,
                items: Vec::new(),
            });
        }
        if let Some(open) = stack.last_mut() {
            open.items
                .push(json!({ "type": "listItem", "content": [item.paragraph] }));
        }
    }
    while !stack.is_empty() {
        close_list(&mut stack, out);
    }
}

fn close_list(stack: &mut Vec<OpenList>, out: &mut Vec<Value>) {
    let Some(open) = stack.pop() else { return };
    let node = json!({
        "type": if open.ordered { "orderedList" } else { "bulletList" },
        "content": open.items,
    });
    let parent_item = stack
        .last_mut()
        .and_then(|parent| parent.items.last_mut())
        .and_then(|item| item.get_mut("content"))
        .and_then(Value::as_array_mut);
    match parent_item {
        Some(content) => content.push(node),
        None => out.push(node),
    }
}

fn paragraph_node(
    document: &Document,
    paragraph: &Paragraph,
    kind: &str,
    heading: Option<u8>,
    report: &mut Report,
) -> Value {
    let mut node = Map::new();
    node.insert("type".into(), kind.into());
    let mut attrs = Map::new();
    if let Some(level) = heading {
        attrs.insert("level".into(), level.into());
    }
    match paragraph.props.align {
        Some(Align::Center) => attrs.insert("textAlign".into(), "center".into()),
        Some(Align::Right) => attrs.insert("textAlign".into(), "right".into()),
        Some(Align::Justify | Align::Distribute) => {
            attrs.insert("textAlign".into(), "justify".into())
        }
        _ => None,
    };
    if let Some(style) = doc_style(document, paragraph) {
        attrs.insert("docStyle".into(), style.into());
    }
    if let Some(LineSpacing::Multiple(multiple)) = paragraph.props.line_spacing {
        attrs.insert("lineHeight".into(), number(f64::from(multiple)));
    }
    for (key, value) in [
        ("spaceBefore", paragraph.props.space_before),
        ("spaceAfter", paragraph.props.space_after),
        ("indent", paragraph.props.indent_left),
    ] {
        if let Some(points) = value.filter(|p| *p > 0.0 && p.is_finite()) {
            attrs.insert(key.into(), number(f64::from(points)));
        }
    }
    if !attrs.is_empty() {
        node.insert("attrs".into(), Value::Object(attrs));
    }
    let content = inline_nodes(paragraph, report);
    if !content.is_empty() {
        node.insert("content".into(), Value::Array(content));
    }
    Value::Object(node)
}

fn inline_nodes(paragraph: &Paragraph, report: &mut Report) -> Vec<Value> {
    let mut out = Vec::new();
    for (range, props) in paragraph.run_ranges() {
        let Some(text) = paragraph.text.get(range) else {
            continue;
        };
        let marks = marks_for(props);
        let mut buffer = String::new();
        let flush = |buffer: &mut String, out: &mut Vec<Value>| {
            if buffer.is_empty() {
                return;
            }
            let mut node = json!({ "type": "text", "text": std::mem::take(buffer) });
            if !marks.is_empty() {
                node["marks"] = Value::Array(marks.clone());
            }
            out.push(node);
        };
        for ch in text.chars() {
            match ch {
                LINE_BREAK => {
                    flush(&mut buffer, &mut out);
                    out.push(json!({ "type": "hardBreak" }));
                }
                OBJ => report.note("Pictures, shapes and other embedded objects were dropped"),
                '\u{000C}' | '\u{000E}' | '\u{00AD}' => {}
                other => buffer.push(other),
            }
        }
        flush(&mut buffer, &mut out);
    }
    out
}

fn marks_for(props: &CharProps) -> Vec<Value> {
    let mut marks = Vec::new();
    if props.bold == Some(true) {
        marks.push(json!({ "type": "bold" }));
    }
    if props.italic == Some(true) {
        marks.push(json!({ "type": "italic" }));
    }
    if props.underline.is_some_and(|u| u != Underline::None) {
        marks.push(json!({ "type": "underline" }));
    }
    if props.strike == Some(true) || props.double_strike == Some(true) {
        marks.push(json!({ "type": "strike" }));
    }
    if let Some(href) = props.link.as_deref().filter(|href| safe_href(href)) {
        marks.push(json!({ "type": "link", "attrs": { "href": href } }));
    }
    if props.vert_align == Some(VertAlign::Superscript) {
        marks.push(json!({ "type": "superscript" }));
    } else if props.vert_align == Some(VertAlign::Subscript) {
        marks.push(json!({ "type": "subscript" }));
    }
    if let Some(color) = props
        .highlight
        .and_then(wordcraft_doc::props::Highlight::rgb)
        .or(props.shading)
    {
        marks.push(json!({ "type": "highlight", "attrs": { "color": css_color(color) } }));
    }
    let mut style = Map::new();
    if let Some(TextColor::Rgb(color)) = props.color {
        style.insert("color".into(), css_color(color).into());
    }
    if let Some(font) = props.font.as_deref() {
        style.insert("fontFamily".into(), font.into());
    }
    if let Some(size) = props.size {
        style.insert("fontSize".into(), format!("{}pt", trim_float(size)).into());
    }
    if !style.is_empty() {
        marks.push(json!({ "type": "textStyle", "attrs": style }));
    }
    marks
}

fn number(value: f64) -> Value {
    json!((value * 100.0).round() / 100.0)
}

fn safe_href(href: &str) -> bool {
    let lower = href.trim().to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://") || lower.starts_with("mailto:")
}

fn css_color(color: wordcraft_doc::Rgb) -> String {
    format!("#{}", color.hex().to_ascii_lowercase())
}

fn trim_float(value: f32) -> String {
    let text = format!("{value:.1}");
    text.strip_suffix(".0")
        .map_or_else(|| text.clone(), str::to_owned)
}

fn table_node(document: &Document, table: &Table, depth: usize, report: &mut Report) -> Value {
    let rows: Vec<Value> = table
        .rows
        .iter()
        .map(|row| {
            let cells: Vec<Value> = row
                .cells
                .iter()
                .map(|cell| {
                    let mut content = if cell.props.vmerge == VMerge::Continue {
                        Vec::new()
                    } else {
                        convert_blocks(
                            document,
                            cell.blocks.iter().map(|b| &**b),
                            depth + 1,
                            report,
                        )
                    };
                    if content.is_empty() {
                        content.push(json!({ "type": "paragraph" }));
                    }
                    let mut node = json!({ "type": "tableCell", "content": content });
                    if cell.span() > 1 {
                        node["attrs"] = json!({ "colspan": cell.span() });
                    }
                    node
                })
                .collect();
            json!({ "type": "tableRow", "content": cells })
        })
        .collect();
    json!({ "type": "table", "content": rows })
}
