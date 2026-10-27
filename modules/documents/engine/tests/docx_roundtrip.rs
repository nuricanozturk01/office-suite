use documents_engine::{ExportError, export_docx, import_docx};
use serde_json::{Value, json};

fn text(text: &str, marks: &Value) -> Value {
    json!({ "type": "text", "text": text, "marks": marks })
}

fn sample() -> Value {
    json!({ "type": "doc", "content": [
        { "type": "heading", "attrs": { "level": 2, "textAlign": "center" },
          "content": [text("Title", &json!([]))] },
        { "type": "paragraph", "content": [
            text("bold", &json!([{ "type": "bold" }])),
            text(" plain ", &json!([])),
            text("link", &json!([{ "type": "link", "attrs": { "href": "https://example.com/" } }])),
            { "type": "hardBreak" },
            text("big red", &json!([{ "type": "textStyle",
                "attrs": { "color": "#ff0000", "fontSize": "18pt", "fontFamily": "Georgia" } }])),
        ] },
        { "type": "bulletList", "content": [
            { "type": "listItem", "content": [
                { "type": "paragraph", "content": [text("one", &json!([]))] },
                { "type": "orderedList", "content": [
                    { "type": "listItem", "content": [
                        { "type": "paragraph", "content": [text("nested", &json!([]))] } ] } ] },
            ] },
            { "type": "listItem", "content": [
                { "type": "paragraph", "content": [text("two", &json!([]))] } ] },
        ] },
        { "type": "table", "content": [
            { "type": "tableRow", "content": [
                { "type": "tableCell", "content": [
                    { "type": "paragraph", "content": [text("a", &json!([]))] } ] },
                { "type": "tableCell", "content": [
                    { "type": "paragraph", "content": [text("b", &json!([]))] } ] } ] },
            { "type": "tableRow", "content": [
                { "type": "tableCell", "attrs": { "colspan": 2 }, "content": [
                    { "type": "paragraph", "content": [text("wide", &json!([]))] } ] } ] },
        ] },
    ] })
}

fn collect_text(node: &Value, out: &mut String) {
    if let Some(t) = node.get("text").and_then(Value::as_str) {
        out.push_str(t);
        out.push('|');
    }
    for child in node
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        collect_text(child, out);
    }
}

fn find<'a>(node: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    if node.get("type").and_then(Value::as_str) == Some(kind) {
        out.push(node);
    }
    for child in node
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        find(child, kind, out);
    }
}

#[test]
fn roundtrip_keeps_structure_and_formatting() {
    let bytes = export_docx(&sample(), "Doc").unwrap();
    assert!(bytes.starts_with(b"PK"));
    let imported = import_docx(&bytes).unwrap();
    let mut all = String::new();
    collect_text(&imported.content, &mut all);
    assert_eq!(
        all,
        "Title|bold| plain |link|big red|one|nested|two|a|b|wide|"
    );

    let mut headings = Vec::new();
    find(&imported.content, "heading", &mut headings);
    assert_eq!(headings[0]["attrs"]["level"], 2);
    assert_eq!(headings[0]["attrs"]["textAlign"], "center");

    let mut bullets = Vec::new();
    find(&imported.content, "bulletList", &mut bullets);
    assert_eq!(bullets.len(), 1);
    let mut ordered = Vec::new();
    find(bullets[0], "orderedList", &mut ordered);
    assert_eq!(ordered.len(), 1);

    let mut cells = Vec::new();
    find(&imported.content, "tableCell", &mut cells);
    assert_eq!(cells.len(), 3);
    assert_eq!(cells[2]["attrs"]["colspan"], 2);

    let rendered = imported.content.to_string();
    for needle in [
        "\"bold\"",
        "https://example.com/",
        "#ff0000",
        "18pt",
        "Georgia",
        "hardBreak",
    ] {
        assert!(rendered.contains(needle), "missing {needle}");
    }
}

#[test]
fn unsafe_link_schemes_are_dropped() {
    let doc = json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [
        text("x", &json!([{ "type": "link", "attrs": { "href": "javascript:alert(1)" } }]))] }] });
    let imported = import_docx(&export_docx(&doc, "t").unwrap()).unwrap();
    assert!(!imported.content.to_string().contains("javascript"));
}

#[test]
fn rejects_garbage_and_non_documents() {
    assert!(import_docx(b"not a zip").is_err());
    assert!(matches!(
        export_docx(&json!({ "type": "paragraph" }), "t"),
        Err(ExportError::NotADocument)
    ));
}

#[test]
fn roundtrip_keeps_named_styles_spacing_and_script_marks() {
    let doc = json!({ "type": "doc", "content": [
        { "type": "paragraph", "attrs": { "docStyle": "title", "lineHeight": 1.5,
            "spaceBefore": 12, "spaceAfter": 6, "indent": 36 },
          "content": [text("Başlık", &json!([{ "type": "superscript" }]))] },
        { "type": "paragraph", "content": [text("alt", &json!([{ "type": "subscript" }]))] },
    ] });
    let imported = import_docx(&export_docx(&doc, "t").unwrap()).unwrap();
    let first = &imported.content["content"][0];
    assert_eq!(first["type"], "paragraph");
    assert_eq!(first["attrs"]["docStyle"], "title");
    assert_eq!(first["attrs"]["lineHeight"], 1.5);
    assert_eq!(first["attrs"]["spaceBefore"], 12.0);
    assert_eq!(first["attrs"]["spaceAfter"], 6.0);
    assert_eq!(first["attrs"]["indent"], 36.0);
    assert_eq!(first["content"][0]["marks"][0]["type"], "superscript");
    assert_eq!(
        imported.content["content"][1]["content"][0]["marks"][0]["type"],
        "subscript"
    );
}
