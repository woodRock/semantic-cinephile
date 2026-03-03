use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Movie {
    pub uuid: String,           // Internal UUID
    pub filepath: String,       // Absolute path
    pub filename: String,       // Original filename
    pub title: String,          // Cleaned title (e.g., "The Matrix")
    pub year: Option<u16>,      // Release year
    pub director: Option<String>,
    pub cast: Vec<String>,      // Top 5 cast members
    pub plot: String,           // Plot summary
    pub keywords: Vec<String>,  // Genres/Keywords
    pub vector: Vec<f32>,       // The semantic embedding of the plot
}

impl Movie {
    pub fn new(filepath: String, filename: String, title: String) -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            filepath,
            filename,
            title,
            year: None,
            director: None,
            cast: vec![],
            plot: String::new(),
            keywords: vec![],
            vector: vec![],
        }
    }
}
