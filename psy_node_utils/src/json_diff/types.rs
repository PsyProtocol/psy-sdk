use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum KeyNode {
    Nil,
    Value(Value, Value),
    Node(HashMap<String, KeyNode>),
}

impl KeyNode {
    pub fn absolute_keys(&self, keys: &mut Vec<String>, key_from_root: Option<String>) {
        let val_key = |key: Option<String>| {
            key.map(|mut s| {
                s.push_str(" ->");
                s
            })
            .unwrap_or(String::new())
        };
        let nil_key = |key: Option<String>| key.unwrap_or(String::new());
        match self {
            KeyNode::Nil => keys.push(nil_key(key_from_root)),
            KeyNode::Value(a, b) => keys.push(format!(
                "{} [ {} :: {} ]",
                val_key(key_from_root),
                a.to_string(),
                b.to_string()
            )),
            KeyNode::Node(map) => {
                for (key, value) in map {
                    value.absolute_keys(
                        keys,
                        Some(format!("{} {}", val_key(key_from_root.clone()), key)),
                    )
                }
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Mismatch {
    pub left_only_keys: KeyNode,
    pub right_only_keys: KeyNode,
    pub keys_in_both: KeyNode,
}

impl Mismatch {
    pub fn new(l: KeyNode, r: KeyNode, u: KeyNode) -> Mismatch {
        Mismatch {
            left_only_keys: l,
            right_only_keys: r,
            keys_in_both: u,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Message {
    BadOption,
    SOURCE1,
    SOURCE2,
    JSON1,
    JSON2,
    UnknownError,
    NoMismatch,
    RootMismatch,
    LeftExtra,
    RightExtra,
    Mismatch,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = match self {
            Message::BadOption => "Invalid option.",
            Message::SOURCE1 => "Could not read source1.",
            Message::SOURCE2 => "Could not read source2.",
            Message::JSON1 => "Could not parse source1.",
            Message::JSON2 => "Could not parse source2.",
            Message::UnknownError => "",
            Message::NoMismatch => "No mismatch was found.",
            Message::RootMismatch => "Mismatch at root.",
            Message::LeftExtra => "Extra on left",
            Message::RightExtra => "Extra on right",
            Message::Mismatch => "Mismatched",
        };

        write!(f, "{}", message)
    }
}