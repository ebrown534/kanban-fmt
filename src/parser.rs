use std::collections::HashMap;

use crate::error::ParseError;
use crate::model::{Board, Card, Column};

/// Parses one kanban export. See README.md for the format this expects.
///
/// The file is line-oriented, so the parser works line by line rather than
/// as a character-level state machine. That keeps line/column bookkeeping
/// for error messages simple: the line number is just the loop index.
///
/// Errors are collected rather than raised on the first bad line: once a
/// line is known to be broken, the parser records a diagnostic and keeps
/// going with a best-effort recovery (an empty name, a skipped card, an
/// empty tag list) so later problems in the same file are also reported.
/// The recovered value is only ever used to keep parsing moving; if
/// `errors` is non-empty at the end, `Err` is returned and the partially
/// built `Board` is discarded.
pub fn parse(source: &str) -> Result<Board, Vec<ParseError>> {
    let lines: Vec<&str> = source.lines().collect();
    let mut errors: Vec<ParseError> = Vec::new();
    let mut i = 0usize;

    while i < lines.len() && lines[i].trim().is_empty() {
        i += 1;
    }

    if i >= lines.len() {
        errors.push(ParseError::new(
            1,
            1,
            1,
            "file is empty, expected a 'board: <name>' header".to_string(),
            None,
        ));
        return Err(errors);
    }

    let board_name = parse_board_header(lines[i], i + 1, &mut errors);
    i += 1;

    let mut columns: Vec<Column> = Vec::new();
    let mut seen_columns: HashMap<String, usize> = HashMap::new();

    while i < lines.len() {
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
            errors.push(ParseError::new(
                line_no,
                char_col(line, indent),
                line.trim_start().chars().count(),
                "expected a column block starting with 'column:'".to_string(),
                Some("blocks are separated by a blank line".to_string()),
            ));
            i += 1;
            continue;
        }

        let column_name = parse_column_header(line, line_no, &mut errors);
        if let Some(&first_line) = seen_columns.get(&column_name) {
            errors.push(ParseError::new(
                line_no,
                char_col(line, indent),
                line.trim_start().chars().count(),
                format!("column '{}' is defined more than once", column_name),
                Some(format!("first defined on line {}", first_line)),
            ));
        } else {
            seen_columns.insert(column_name.clone(), line_no);
        }
        i += 1;

        let mut cards: Vec<Card> = Vec::new();
        while i < lines.len() && !lines[i].trim().is_empty() {
            if let Some(card) = parse_card_line(lines[i], i + 1, &mut errors) {
                cards.push(card);
            }
            i += 1;
        }

        columns.push(Column {
            name: column_name,
            cards,
        });
    }

    if columns.is_empty() {
        let last_line = lines.len().max(1);
        errors.push(ParseError::new(
            last_line,
            1,
            1,
            "board has no columns".to_string(),
            Some("add at least one 'column: <name>' block".to_string()),
        ));
    }

    if !errors.is_empty() {
        return Err(errors);
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

fn parse_board_header(line: &str, line_no: usize, errors: &mut Vec<ParseError>) -> String {
    let indent = line.len() - line.trim_start().len();
    let trimmed = line.trim_start();

    if !trimmed.starts_with("board:") {
        errors.push(ParseError::new(
            line_no,
            1,
            line.chars().count(),
            "expected 'board: <name>' as the first line of the file".to_string(),
            Some("every kanban export starts with a board header".to_string()),
        ));
        return String::new();
    }

    let after = &trimmed["board:".len()..];
    let name = after.trim();
    if name.is_empty() {
        errors.push(ParseError::new(
            line_no,
            char_col(line, indent + "board:".len()),
            1,
            "board name cannot be empty".to_string(),
            None,
        ));
    }
    name.to_string()
}

fn parse_column_header(line: &str, line_no: usize, errors: &mut Vec<ParseError>) -> String {
    let indent = line.len() - line.trim_start().len();
    let trimmed = line.trim_start();
    let after = &trimmed["column:".len()..];
    let name = after.trim();
    if name.is_empty() {
        errors.push(ParseError::new(
            line_no,
            char_col(line, indent + "column:".len()),
            1,
            "column name cannot be empty".to_string(),
            None,
        ));
    }
    name.to_string()
}

fn parse_card_line(line: &str, line_no: usize, errors: &mut Vec<ParseError>) -> Option<Card> {
    let indent = line.len() - line.trim_start().len();
    let trimmed = line.trim_start();

    if !trimmed.starts_with('-') {
        errors.push(ParseError::new(
            line_no,
            char_col(line, indent),
            trimmed.chars().count().max(1),
            "expected a card line starting with '-'".to_string(),
            Some("columns must be separated by a blank line".to_string()),
        ));
        return None;
    }

    let after_dash = &trimmed[1..];
    if !after_dash.starts_with(' ') {
        errors.push(ParseError::new(
            line_no,
            char_col(line, indent + 1),
            1,
            "expected a space after '-'".to_string(),
            None,
        ));
        return None;
    }

    let spaces = after_dash.len() - after_dash.trim_start().len();
    let rest = after_dash.trim_start();
    let rest_start = indent + 1 + spaces;

    parse_card_body(rest, rest_start, line, line_no, errors)
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
    errors: &mut Vec<ParseError>,
) -> Option<Card> {
    let marker_start = find_marker_start(rest);
    let title_part = match marker_start {
        Some(m) => &rest[..m],
        None => rest,
    };
    let title = title_part.trim_end().to_string();
    let mut card_ok = true;
    if title.is_empty() {
        errors.push(ParseError::new(
            line_no,
            char_col(line, rest_start),
            1,
            "card title cannot be empty".to_string(),
            None,
        ));
        card_ok = false;
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
                        errors.push(ParseError::new(
                            line_no,
                            col,
                            len,
                            "assignee name cannot be empty after '@'".to_string(),
                            None,
                        ));
                        card_ok = false;
                    } else if assignee.is_some() {
                        errors.push(ParseError::new(
                            line_no,
                            col,
                            len,
                            "card already has an assignee".to_string(),
                            Some("a card can only be assigned to one person".to_string()),
                        ));
                        card_ok = false;
                    } else {
                        assignee = Some(name.to_string());
                    }
                }
                Some('#') => {
                    let list = &token[1..];
                    if list.is_empty() {
                        errors.push(ParseError::new(
                            line_no,
                            col,
                            len,
                            "tag list cannot be empty after '#'".to_string(),
                            None,
                        ));
                        card_ok = false;
                        continue;
                    }
                    if !tags.is_empty() {
                        errors.push(ParseError::new(
                            line_no,
                            col,
                            len,
                            "duplicate tag list".to_string(),
                            Some("combine tags into one '#tag1,tag2' group".to_string()),
                        ));
                        card_ok = false;
                        continue;
                    }
                    let mut parsed_tags = Vec::new();
                    for part in list.split(',') {
                        if part.trim().is_empty() {
                            errors.push(ParseError::new(
                                line_no,
                                col,
                                len,
                                "empty tag between commas".to_string(),
                                None,
                            ));
                            card_ok = false;
                            parsed_tags.clear();
                            break;
                        }
                        parsed_tags.push(part.trim().to_string());
                    }
                    tags = parsed_tags;
                }
                _ => {
                    errors.push(ParseError::new(
                        line_no,
                        col,
                        len,
                        format!("unexpected text '{}' in card metadata", token),
                        Some("expected '@name' for an assignee or '#tag' for tags".to_string()),
                    ));
                    card_ok = false;
                }
            }
        }
    }

    if card_ok {
        Some(Card {
            title,
            assignee,
            tags,
        })
    } else {
        None
    }
}
