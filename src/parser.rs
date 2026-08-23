use std::collections::HashMap;

use crate::error::ParseError;
use crate::model::{Board, Card, Column};

/// Parses one kanban export. See README.md for the format this expects.
///
/// The file is line-oriented, so the parser works line by line rather than
/// as a character-level state machine. That keeps line/column bookkeeping
/// for error messages simple: the line number is just the loop index.
pub fn parse(source: &str) -> Result<Board, ParseError> {
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0usize;

    while i < lines.len() && lines[i].trim().is_empty() {
        i += 1;
    }

    if i >= lines.len() {
        return Err(ParseError::new(
            1,
            1,
            1,
            "file is empty, expected a 'board: <name>' header".to_string(),
            None,
        ));
    }

    let board_name = parse_board_header(lines[i], i + 1)?;
    i += 1;

    let mut columns: Vec<Column> = Vec::new();
    let mut seen_columns: HashMap<String, usize> = HashMap::new();

    loop {
        while i < lines.len() && lines[i].trim().is_empty() {
            i += 1;
        }
        if i >= lines.len() {
            break;
        }

        let line = lines[i];
        let line_no = i + 1;
        let indent = line.len() - line.trim_start().len();

        if !line.trim_start().starts_with("column:") {
            return Err(ParseError::new(
                line_no,
                char_col(line, indent),
                line.trim_start().chars().count(),
                "expected a column block starting with 'column:'".to_string(),
                Some("blocks are separated by a blank line".to_string()),
            ));
        }

        let column_name = parse_column_header(line, line_no)?;
        if let Some(&first_line) = seen_columns.get(&column_name) {
            return Err(ParseError::new(
                line_no,
                char_col(line, indent),
                line.trim_start().chars().count(),
                format!("column '{}' is defined more than once", column_name),
                Some(format!("first defined on line {}", first_line)),
            ));
        }
        seen_columns.insert(column_name.clone(), line_no);
        i += 1;

        let mut cards: Vec<Card> = Vec::new();
        while i < lines.len() && !lines[i].trim().is_empty() {
            let card = parse_card_line(lines[i], i + 1)?;
            cards.push(card);
            i += 1;
        }

        columns.push(Column {
            name: column_name,
            cards,
        });
    }

    if columns.is_empty() {
        let last_line = lines.len().max(1);
        return Err(ParseError::new(
            last_line,
            1,
            1,
            "board has no columns".to_string(),
            Some("add at least one 'column: <name>' block".to_string()),
        ));
    }

    Ok(Board {
        name: board_name,
        columns,
    })
}

/// Converts a byte offset within `line` to a 1-based character column.
/// Errors are reported in characters, not bytes, so a caret still lands on
/// the right character when the line has multi-byte UTF-8 content before it.
fn char_col(line: &str, byte_idx: usize) -> usize {
    line.get(..byte_idx).unwrap_or("").chars().count() + 1
}

fn parse_board_header(line: &str, line_no: usize) -> Result<String, ParseError> {
    let indent = line.len() - line.trim_start().len();
    let trimmed = line.trim_start();

    if !trimmed.starts_with("board:") {
        return Err(ParseError::new(
            line_no,
            1,
            line.chars().count(),
            "expected 'board: <name>' as the first line of the file".to_string(),
            Some("every kanban export starts with a board header".to_string()),
        ));
    }

    let after = &trimmed["board:".len()..];
    let name = after.trim();
    if name.is_empty() {
        return Err(ParseError::new(
            line_no,
            char_col(line, indent + "board:".len()),
            1,
            "board name cannot be empty".to_string(),
            None,
        ));
    }
    Ok(name.to_string())
}

fn parse_column_header(line: &str, line_no: usize) -> Result<String, ParseError> {
    let indent = line.len() - line.trim_start().len();
    let trimmed = line.trim_start();
    let after = &trimmed["column:".len()..];
    let name = after.trim();
    if name.is_empty() {
        return Err(ParseError::new(
            line_no,
            char_col(line, indent + "column:".len()),
            1,
            "column name cannot be empty".to_string(),
            None,
        ));
    }
    Ok(name.to_string())
}

fn parse_card_line(line: &str, line_no: usize) -> Result<Card, ParseError> {
    let indent = line.len() - line.trim_start().len();
    let trimmed = line.trim_start();

    if !trimmed.starts_with('-') {
        return Err(ParseError::new(
            line_no,
            char_col(line, indent),
            trimmed.chars().count().max(1),
            "expected a card line starting with '-'".to_string(),
            Some("columns must be separated by a blank line".to_string()),
        ));
    }

    let after_dash = &trimmed[1..];
    if !after_dash.starts_with(' ') {
        return Err(ParseError::new(
            line_no,
            char_col(line, indent + 1),
            1,
            "expected a space after '-'".to_string(),
            None,
        ));
    }

    let spaces = after_dash.len() - after_dash.trim_start().len();
    let rest = after_dash.trim_start();
    let rest_start = indent + 1 + spaces;

    let (title, assignee, tags) = parse_card_body(rest, rest_start, line, line_no)?;

    Ok(Card {
        title,
        assignee,
        tags,
    })
}

/// A card's metadata markers (`@assignee`, `#tag,tag`) only count if they
/// start at the beginning of the remaining text or are preceded by
/// whitespace. That lets a title contain a literal '@' or '#' as long as it
/// isn't at a word boundary, e.g. "Fix login@prod issue".
fn find_marker_start(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    for (idx, c) in rest.char_indices() {
        if (c == '@' || c == '#') && (idx == 0 || bytes[idx - 1] == b' ') {
            return Some(idx);
        }
    }
    None
}

fn tokenize(s: &str) -> Vec<(usize, &str)> {
    let mut result = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in s.char_indices() {
        if c.is_whitespace() {
            if let Some(st) = start {
                result.push((st, &s[st..i]));
                start = None;
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(st) = start {
        result.push((st, &s[st..]));
    }
    result
}

fn parse_card_body(
    rest: &str,
    rest_start: usize,
    line: &str,
    line_no: usize,
) -> Result<(String, Option<String>, Vec<String>), ParseError> {
    let marker_start = find_marker_start(rest);
    let title_part = match marker_start {
        Some(m) => &rest[..m],
        None => rest,
    };
    let title = title_part.trim_end().to_string();
    if title.is_empty() {
        return Err(ParseError::new(
            line_no,
            char_col(line, rest_start),
            1,
            "card title cannot be empty".to_string(),
            None,
        ));
    }

    let mut assignee: Option<String> = None;
    let mut tags: Vec<String> = Vec::new();

    if let Some(m) = marker_start {
        let metadata = &rest[m..];
        for (tok_off, token) in tokenize(metadata) {
            let abs_byte = rest_start + m + tok_off;
            let col = char_col(line, abs_byte);
            let len = token.chars().count().max(1);

            match token.chars().next() {
                Some('@') => {
                    let name = &token[1..];
                    if name.is_empty() {
                        return Err(ParseError::new(
                            line_no,
                            col,
                            len,
                            "assignee name cannot be empty after '@'".to_string(),
                            None,
                        ));
                    }
                    if assignee.is_some() {
                        return Err(ParseError::new(
                            line_no,
                            col,
                            len,
                            "card already has an assignee".to_string(),
                            Some("a card can only be assigned to one person".to_string()),
                        ));
                    }
                    assignee = Some(name.to_string());
                }
                Some('#') => {
                    let list = &token[1..];
                    if list.is_empty() {
                        return Err(ParseError::new(
                            line_no,
                            col,
                            len,
                            "tag list cannot be empty after '#'".to_string(),
                            None,
                        ));
                    }
                    if !tags.is_empty() {
                        return Err(ParseError::new(
                            line_no,
                            col,
                            len,
                            "duplicate tag list".to_string(),
                            Some("combine tags into one '#tag1,tag2' group".to_string()),
                        ));
                    }
                    for part in list.split(',') {
                        if part.trim().is_empty() {
                            return Err(ParseError::new(
                                line_no,
                                col,
                                len,
                                "empty tag between commas".to_string(),
                                None,
                            ));
                        }
                        tags.push(part.trim().to_string());
                    }
                }
                _ => {
                    return Err(ParseError::new(
                        line_no,
                        col,
                        len,
                        format!("unexpected text '{}' in card metadata", token),
                        Some("expected '@name' for an assignee or '#tag' for tags".to_string()),
                    ));
                }
            }
        }
    }

    Ok((title, assignee, tags))
}
