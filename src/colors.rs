pub const COLORS: [&'static str; 12] = [
  "0;31",
  "0;32",
  "0;33",
  "0;34",
  "0;35",
  "0;36",
  "1;31",
  "1;32",
  "1;33",
  "1;34",
  "1;35",
  "1;36"
];

pub fn colorize(text: &str, color: &str) -> String {
    let esc_char = vec![27];
    let esc = String::from_utf8(esc_char).unwrap();
    format!("{}[{}m{}{}[0m", esc, color, text, esc)
}
