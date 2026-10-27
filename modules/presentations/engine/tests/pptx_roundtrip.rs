use presentations_engine::{ExportError, export_pptx, import_pptx};
use serde_json::{Value, json};

fn sample() -> Value {
    json!({
        "width": 960, "height": 540,
        "slides": [
            { "id": "s1", "background": "#112233", "notes": "speaker notes", "elements": [
                { "id": "e1", "kind": "text", "x": 40, "y": 30, "w": 500, "h": 80,
                  "text": "Merhaba dünya\nikinci satır", "fontSize": 32, "bold": true,
                  "color": "#ff0000", "align": "center" },
                { "id": "e2", "kind": "shape", "shape": "ellipse", "x": 100, "y": 200,
                  "w": 200, "h": 120, "fill": "#00ff00", "stroke": "#0000ff", "strokeWidth": 3,
                  "text": "içeride", "rotation": 15 },
                { "id": "e3", "kind": "shape", "shape": "line", "x": 10, "y": 400, "w": 300, "h": 0,
                  "stroke": "#000000", "strokeWidth": 2 }
            ] },
            { "id": "s2", "elements": [] }
        ]
    })
}

#[test]
fn roundtrip_keeps_slides_text_and_geometry() {
    let bytes = export_pptx(&sample()).unwrap();
    assert!(bytes.starts_with(b"PK"));
    let imported = import_pptx(&bytes).unwrap();
    let deck = &imported.deck;
    assert_eq!(deck["width"], 960.0);
    assert_eq!(deck["slides"].as_array().unwrap().len(), 2);
    let first = &deck["slides"][0];
    assert_eq!(first["background"], "#112233");
    assert_eq!(first["notes"], "speaker notes");
    let elements = first["elements"].as_array().unwrap();
    assert_eq!(elements.len(), 3);
    assert_eq!(elements[0]["kind"], "text");
    assert_eq!(elements[0]["text"], "Merhaba dünya\nikinci satır");
    assert_eq!(elements[0]["fontSize"], 32.0);
    assert_eq!(elements[0]["bold"], true);
    assert_eq!(elements[0]["color"], "#ff0000");
    assert_eq!(elements[0]["align"], "center");
    assert_eq!(elements[0]["x"], 40.0);
    assert_eq!(elements[1]["shape"], "ellipse");
    assert_eq!(elements[1]["fill"], "#00ff00");
    assert_eq!(elements[1]["stroke"], "#0000ff");
    assert_eq!(elements[1]["text"], "içeride");
    assert_eq!(elements[1]["rotation"], 15.0);
    assert_eq!(elements[2]["shape"], "line");
}

#[test]
fn rejects_garbage_and_non_decks() {
    assert!(import_pptx(b"not a zip").is_err());
    assert!(matches!(
        export_pptx(&json!({})),
        Err(ExportError::NotADeck)
    ));
}
