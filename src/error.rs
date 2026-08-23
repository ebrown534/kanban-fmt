/// A single parse failure, anchored to a line/column in the source file.
///
/// `len` is measured in characters, not bytes, so the caret underline lines
/// up correctly even when the source contains multi-byte UTF-8 text.
pub struct ParseError {
    pub line: usize,
    pub col: usize,
    pub len: usize,
    pub message: String,
    pub note: Option<String>,
}

impl ParseError {
    pub fn new(line: usize, col: usize, len: usize, message: String, note: Option<String>) -> Self {
        ParseError {
            line,
            col,
            len: len.max(1),
            message,
            note,
        }
    }

    /// Render this error the way rustc renders its own diagnostics: a
    /// message, a file:line:col pointer, the offending source line, and a
    /// caret underline. That format is the whole point of this crate, so it
    /// gets its own function instead of being folded into Display.
    pub fn render(&self, filename: &str, source: &str) -> String {
        let line_text = source.lines().nth(self.line.saturating_sub(1)).unwrap_or("");
        let gutter = self.line.to_string();
        let pad = " ".repeat(gutter.len());
        let caret_offset = self.col.saturating_sub(1);

        let mut out = String::new();
        out.push_str(&format!("error: {}\n", self.message));
        out.push_str(&format!("{}--> {}:{}:{}\n", pad, filename, self.line, self.col));
        out.push_str(&format!("{} |\n", pad));
        out.push_str(&format!("{} | {}\n", gutter, line_text));
        out.push_str(&format!(
            "{} | {}{}\n",
            pad,
            " ".repeat(caret_offset),
            "^".repeat(self.len)
        ));
        if let Some(note) = &self.note {
            out.push_str(&format!("{} = note: {}\n", pad, note));
        }
        out
    }
}
