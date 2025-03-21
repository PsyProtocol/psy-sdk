pub const BUILTINS: &[&str] = &[
    "FELT",
    "BOOL",
];

pub fn get_builtin_description(rule: &str) -> Option<&str> {
    match rule {
        "FELT" => Some("Built-in type Felt"),
        "BOOL" => Some("Built-in type Bool"),
      _ => None,
    }
}
