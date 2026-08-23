#[derive(Debug, Clone)]
pub struct Board {
    pub name: String,
    pub columns: Vec<Column>,
}

#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub cards: Vec<Card>,
}

#[derive(Debug, Clone)]
pub struct Card {
    pub title: String,
    pub assignee: Option<String>,
    pub tags: Vec<String>,
}
