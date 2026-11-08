use serde::Deserialize;
use serde_json::Value;

const PALETTE: [(&str, (u8, u8, u8)); 16] = [
    ("black", (0x00, 0x00, 0x00)),
    ("dark_blue", (0x00, 0x00, 0xaa)),
    ("dark_green", (0x00, 0xaa, 0x00)),
    ("dark_aqua", (0x00, 0xaa, 0xaa)),
    ("dark_red", (0xaa, 0x00, 0x00)),
    ("dark_purple", (0xaa, 0x00, 0xaa)),
    ("gold", (0xff, 0xaa, 0x00)),
    ("gray", (0xaa, 0xaa, 0xaa)),
    ("dark_gray", (0x55, 0x55, 0x55)),
    ("blue", (0x55, 0x55, 0xff)),
    ("green", (0x55, 0xff, 0x55)),
    ("aqua", (0x55, 0xff, 0xff)),
    ("red", (0xff, 0x55, 0x55)),
    ("light_purple", (0xff, 0x55, 0xff)),
    ("yellow", (0xff, 0xff, 0x55)),
    ("white", (0xff, 0xff, 0xff)),
];

#[derive(Deserialize)]
pub struct Status {
    pub version: Version,
    pub players: Option<Players>,
    #[serde(default)]
    pub description: Value,
    pub favicon: Option<String>,
}

#[derive(Deserialize)]
pub struct Version {
    pub name: String,
    pub protocol: i32,
}

#[derive(Deserialize)]
pub struct Players {
    pub max: i64,
    pub online: i64,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Style {
    pub color: Option<(u8, u8, u8)>,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Debug, PartialEq)]
pub struct Span {
    pub text: String,
    pub style: Style,
}

pub fn spans(value: &Value) -> Vec<Span> {
    let mut out = Vec::new();
    walk(value, &Style::default(), &mut out);
    out
}

pub fn plain(spans: &[Span]) -> String {
    spans.iter().map(|span| span.text.as_str()).collect()
}

fn walk(value: &Value, parent: &Style, out: &mut Vec<Span>) {
    match value {
        Value::String(text) => push_legacy(text, parent.clone(), out),
        Value::Array(items) => {
            for item in items {
                walk(item, parent, out);
            }
        }
        Value::Object(map) => {
            let mut style = parent.clone();
            if let Some(color) = map.get("color").and_then(Value::as_str) {
                style.color = parse_color(color);
            }
            if let Some(bold) = map.get("bold").and_then(Value::as_bool) {
                style.bold = bold;
            }
            if let Some(italic) = map.get("italic").and_then(Value::as_bool) {
                style.italic = italic;
            }
            if let Some(Value::String(text)) = map.get("text") {
                push_legacy(text, style.clone(), out);
            }
            if let Some(extra) = map.get("extra") {
                walk(extra, &style, out);
            }
        }
        _ => {}
    }
}

fn push_legacy(text: &str, mut style: Style, out: &mut Vec<Span>) {
    let mut buf = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '§' {
            buf.push(c);
            continue;
        }
        let Some(code) = chars.next() else { break };
        if !buf.is_empty() {
            out.push(Span {
                text: std::mem::take(&mut buf),
                style: style.clone(),
            });
        }
        match code.to_ascii_lowercase() {
            'l' => style.bold = true,
            'o' => style.italic = true,
            'r' => style = Style::default(),
            code => {
                if let Some(i) = code.to_digit(16) {
                    style = Style {
                        color: Some(PALETTE[i as usize].1),
                        ..Style::default()
                    };
                }
            }
        }
    }
    if !buf.is_empty() {
        out.push(Span { text: buf, style });
    }
}

fn parse_color(color: &str) -> Option<(u8, u8, u8)> {
    if let Some(hex) = color.strip_prefix('#') {
        let rgb = u32::from_str_radix(hex, 16).ok()?;
        return Some(((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8));
    }
    PALETTE
        .iter()
        .find(|(name, _)| *name == color)
        .map(|(_, rgb)| *rgb)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn legacy_codes_split_into_colored_spans() {
        let spans = spans(&json!("§aHello §l§cWorld"));
        assert_eq!(plain(&spans), "Hello World");
        assert_eq!(spans[0].style.color, Some((0x55, 0xff, 0x55)));
        assert_eq!(spans[1].style.color, Some((0xff, 0x55, 0x55)));
        assert!(!spans[1].style.bold);
    }

    #[test]
    fn components_inherit_parent_style() {
        let motd = json!({
            "text": "A ",
            "color": "gold",
            "bold": true,
            "extra": [{ "text": "B" }, { "text": "C", "color": "#123456", "bold": false }]
        });
        let spans = spans(&motd);
        assert_eq!(plain(&spans), "A BC");
        assert_eq!(spans[1].style.color, Some((0xff, 0xaa, 0x00)));
        assert!(spans[1].style.bold);
        assert_eq!(spans[2].style.color, Some((0x12, 0x34, 0x56)));
        assert!(!spans[2].style.bold);
    }

    #[test]
    fn parses_status_without_players() {
        let status: Status =
            serde_json::from_value(json!({ "version": { "name": "1.21", "protocol": 767 } }))
                .unwrap();
        assert!(status.players.is_none());
        assert!(spans(&status.description).is_empty());
    }
}
