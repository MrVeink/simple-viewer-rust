use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum DataSource {
    Local(PathBuf),
    Cloud(String, String),  // (url, sheet_name)
}

#[derive(Debug, Clone)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl TableData {
    pub fn empty() -> Self {
        TableData {
            headers: Vec::new(),
            rows: Vec::new(),
        }
    }
}
