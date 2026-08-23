use crate::model::Board;

/// Renders a `Board` back to the canonical text form: one space after '-',
/// a single space before '@'/'#' markers, and tags joined with no spaces
/// after the commas. Round-tripping a valid file through parse + this
/// should be a no-op once the input is already canonical.
pub fn pretty_print(board: &Board) -> String {
    let mut out = String::new();
    out.push_str("board: ");
    out.push_str(&board.name);
    out.push('\n');

    for column in &board.columns {
        out.push('\n');
        out.push_str("column: ");
        out.push_str(&column.name);
        out.push('\n');

        for card in &column.cards {
            out.push_str("- ");
            out.push_str(&card.title);
            if let Some(assignee) = &card.assignee {
                out.push_str(" @");
                out.push_str(assignee);
            }
            if !card.tags.is_empty() {
                out.push_str(" #");
                out.push_str(&card.tags.join(","));
            }
            out.push('\n');
        }
    }

    out
}
