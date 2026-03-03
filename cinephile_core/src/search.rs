use anyhow::Result;
use crate::models::Movie;
use crate::store::Store;
use crate::embedder::Embedder;
use ollama_rs::{Ollama, generation::completion::request::GenerationRequest};
use serde::{Deserialize, Serialize};

pub struct Searcher {
    store: Store,
    embedder: Embedder,
    ollama: Ollama,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchIntent {
    is_semantic: bool,
    filters: Option<SearchFilters>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchFilters {
    director: Option<String>,
    year: Option<u16>,
}

impl Searcher {
    pub fn new(store: Store, embedder: Embedder) -> Self {
        Self {
            store,
            embedder,
            ollama: Ollama::default(),
        }
    }

    pub async fn search(&mut self, query_str: &str) -> Result<Vec<Movie>> {
        // Step 1: Intent Classification
        let intent = self.classify_intent(query_str).await?;
        
        // Step 2: Generate Embedding
        let vector = self.embedder.embed(query_str)?;

        // Step 3: Build Filter
        let mut filter_str = None;
        if let Some(filters) = intent.filters {
            let mut parts = Vec::new();
            if let Some(d) = filters.director {
                parts.push(format!("director = '{}'", d.replace("'", "''")));
            }
            if let Some(y) = filters.year {
                parts.push(format!("year = {}", y));
            }
            if !parts.is_empty() {
                filter_str = Some(parts.join(" AND "));
            }
        }

        // Step 4: Search
        self.store.hybrid_search(vector, filter_str).await
    }

    async fn classify_intent(&self, query: &str) -> Result<SearchIntent> {
        let prompt = format!(
            "Analyze the movie search query: '{}'. Return JSON: {{ \"is_semantic\": bool, \"filters\": {{ \"director\": string or null, \"year\": int or null }} }}",
            query
        );

        match self.ollama.generate(GenerationRequest::new("llama3.2".to_string(), prompt)).await {
            Ok(res) => {
                let response_text = res.response;
                if let Some(start) = response_text.find('{') {
                    if let Some(end) = response_text.rfind('}') {
                        let json_str = &response_text[start..=end];
                        if let Ok(intent) = serde_json::from_str::<SearchIntent>(json_str) {
                            return Ok(intent);
                        }
                    }
                }
            }
            Err(_) => {}
        }

        // Default: semantic search, no filters
        Ok(SearchIntent {
            is_semantic: true,
            filters: None,
        })
    }
}
