use anyhow::{Result, Context};
use crate::models::Movie;
use surrealdb::engine::local::Db;
use surrealdb::engine::local::RocksDb;
use surrealdb::Surreal;
use directories::ProjectDirs;
use std::fs;

pub struct Store {
    db: Surreal<Db>,
}

impl Store {
    pub async fn new() -> Result<Self> {
        let proj_dirs = ProjectDirs::from("com", "cinephile", "cinephile")
            .context("Failed to get project directories")?;
        let data_dir = proj_dirs.data_dir();
        fs::create_dir_all(data_dir)?;
        
        let db_path = data_dir.join("surreal");
        
        let db = Surreal::new::<RocksDb>(db_path.to_str().unwrap()).await?;
        
        db.use_ns("cinephile").use_db("movies").await?;
        
        Ok(Self { db })
    }

    pub async fn upsert_movie(&self, movie: Movie) -> Result<()> {
        let id = urlencoding::encode(&movie.filepath).replace("%", "");
        let _: Option<serde_json::Value> = self.db.update(("movie", &id))
            .content(movie)
            .await?;
        Ok(())
    }

    pub async fn hybrid_search(&self, vector: Vec<f32>, filter_str: Option<String>) -> Result<Vec<Movie>> {
        let mut query_str = "SELECT * FROM movie".to_string();
        if let Some(f) = filter_str {
            query_str.push_str(&format!(" WHERE {}", f));
        }

        let mut results = self.db.query(query_str).await?;
        let movies: Vec<Movie> = results.take(0)?;

        let mut scored_movies: Vec<(f32, Movie)> = movies.into_iter()
            .map(|m| (cosine_similarity(&vector, &m.vector), m))
            .collect();

        scored_movies.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored_movies.into_iter().take(10).map(|(_, m)| m).collect())
    }
}

fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
    if v1.is_empty() || v2.is_empty() || v1.len() != v2.len() {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut norm1 = 0.0;
    let mut norm2 = 0.0;
    for i in 0..v1.len() {
        dot += v1[i] * v2[i];
        norm1 += v1[i] * v1[i];
        norm2 += v2[i] * v2[i];
    }
    dot / (norm1.sqrt() * norm2.sqrt())
}
