//! A YAML tree that keeps every node's position, so a config error can name
//! `file:line:col` and its key path. Scalars stay strings; the config layer
//! reads types from them.

use granit_parser::{Event, Parser, ScalarStyle, Span};

/// 1-based line and column (code points).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub enum Value {
    Map(Vec<(Key, Node)>),
    Seq(Vec<Node>),
    /// A quoted or block scalar: always a string.
    Str(String),
    /// A plain scalar: a number, boolean or null, or else a string.
    Plain(String),
    Null,
}

#[derive(Debug, Clone)]
pub struct Key {
    pub name: String,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub value: Value,
    pub pos: Pos,
}

impl Node {
    /// The text of a scalar that reads as a string (not a number, bool or null).
    pub fn as_str(&self) -> Option<&str> {
        match &self.value {
            Value::Str(s) => Some(s),
            Value::Plain(s) if plain_kind(s) == PlainKind::Str => Some(s),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match &self.value {
            Value::Plain(s) if plain_kind(s) == PlainKind::Number => s.parse().ok(),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match &self.value {
            Value::Plain(s) if plain_kind(s) == PlainKind::Bool => Some(s == "true"),
            _ => None,
        }
    }

    /// What the node is, for "expected …, got …" messages.
    pub fn kind(&self) -> &'static str {
        match &self.value {
            Value::Map(_) => "a mapping",
            Value::Seq(_) => "a list",
            Value::Str(_) => "a string",
            Value::Plain(s) => match plain_kind(s) {
                PlainKind::Number => "a number",
                PlainKind::Bool => "a boolean",
                PlainKind::Str => "a string",
            },
            Value::Null => "nothing",
        }
    }
}

#[derive(PartialEq)]
enum PlainKind {
    Number,
    Bool,
    Str,
}

fn plain_kind(s: &str) -> PlainKind {
    if s == "true" || s == "false" {
        PlainKind::Bool
    } else if s.parse::<f64>().is_ok() && !s.contains(['n', 'N', 'i', 'I']) {
        PlainKind::Number
    } else {
        PlainKind::Str
    }
}

fn pos(span: &Span) -> Pos {
    Pos {
        line: span.start.line(),
        col: span.start.col() + 1,
    }
}

/// Parse one YAML document. An empty document is `Null`.
pub fn parse(text: &str) -> Result<Node, (Pos, String)> {
    let mut events = Parser::new_from_str(text).map(|item| {
        item.map_err(|e| {
            let at = Pos {
                line: e.marker().line(),
                col: e.marker().col() + 1,
            };
            (at, e.info())
        })
    });
    let mut root = Node {
        value: Value::Null,
        pos: Pos { line: 1, col: 1 },
    };
    let mut documents = 0;
    while let Some(item) = events.next() {
        let (event, span) = item?;
        match event {
            Event::StreamStart | Event::StreamEnd | Event::DocumentEnd => {}
            Event::DocumentStart(..) => {
                documents += 1;
                if documents > 1 {
                    return Err((pos(&span), "only one YAML document is allowed".into()));
                }
            }
            _ => root = node(event, span, &mut events)?,
        }
    }
    Ok(root)
}

type Item<'a> = Result<(Event<'a>, Span), (Pos, String)>;

fn node<'a>(
    event: Event<'a>,
    span: Span,
    events: &mut impl Iterator<Item = Item<'a>>,
) -> Result<Node, (Pos, String)> {
    let at = pos(&span);
    let value = match event {
        Event::Scalar(text, style, _, _) => match style {
            ScalarStyle::Plain if matches!(&*text, "" | "~" | "null") => Value::Null,
            ScalarStyle::Plain => Value::Plain(text.into_owned()),
            _ => Value::Str(text.into_owned()),
        },
        Event::SequenceStart(..) => {
            let mut items = Vec::new();
            loop {
                let (event, span) = next(events, at)?;
                if matches!(event, Event::SequenceEnd) {
                    break;
                }
                items.push(node(event, span, events)?);
            }
            Value::Seq(items)
        }
        Event::MappingStart(..) => {
            let mut entries: Vec<(Key, Node)> = Vec::new();
            loop {
                let (event, span) = next(events, at)?;
                if matches!(event, Event::MappingEnd) {
                    break;
                }
                let key_pos = pos(&span);
                let Event::Scalar(name, ..) = event else {
                    return Err((key_pos, "a mapping key must be a plain name".into()));
                };
                let name = name.into_owned();
                if entries.iter().any(|(k, _)| k.name == name) {
                    return Err((key_pos, format!("duplicate key `{name}`")));
                }
                let (event, span) = next(events, at)?;
                let value = node(event, span, events)?;
                entries.push((Key { name, pos: key_pos }, value));
            }
            Value::Map(entries)
        }
        Event::Alias(_) => return Err((at, "anchors and aliases aren't supported".into())),
        _ => return Err((at, "unexpected YAML structure".into())),
    };
    Ok(Node { value, pos: at })
}

fn next<'a>(
    events: &mut impl Iterator<Item = Item<'a>>,
    at: Pos,
) -> Result<(Event<'a>, Span), (Pos, String)> {
    events
        .next()
        .unwrap_or_else(|| Err((at, "unexpected end of YAML".into())))
}
