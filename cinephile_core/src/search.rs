use anyhow::Result;
use crate::models::Movie;
use crate::store::Store;
use crate::embedder::Embedder;
use ollama_rs::{
    Ollama, 
    generation::completion::request::GenerationRequest,
    generation::parameters::FormatType
};
use serde::{Deserialize, Serialize};

pub struct Searcher {
    store: Store,
    embedder: Embedder,
    ollama: Ollama,
    model: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "tool", content = "params")]
enum SearchTool {
    #[serde(rename = "semantic")]
    Semantic { query: String },
    
    #[serde(rename = "similarity")]
    Similarity { title: String },
    
    #[serde(rename = "metadata")]
    Metadata { filters: SearchFilters },
    
    #[serde(rename = "hybrid")]
    Hybrid { query: String, filters: SearchFilters },
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct SearchFilters {
    director: Option<String>,
    cast: Option<String>,
    year: Option<u16>,
    keyword: Option<String>,
}

impl Searcher {
    pub fn new(store: Store, embedder: Embedder) -> Self {
        Self {
            store,
            embedder,
            ollama: Ollama::default(),
            model: "qwen3.5:0.8b".to_string(),
        }
    }

    pub fn set_model(&mut self, model: String) {
        self.model = model;
    }

    pub async fn search(&mut self, query_str: &str) -> Result<Vec<Movie>> {
        // Step 1: Tool Selection (Intent Classification)
        let tool = self.select_tool(query_str).await?;
        
        match tool {
            SearchTool::Semantic { query } => {
                let vector = self.embedder.embed(&query)?;
                self.store.hybrid_search(Some(vector), None).await
            }
            SearchTool::Similarity { title } => {
                if let Some(movie) = self.store.find_by_title(&title).await? {
                    self.store.hybrid_search(Some(movie.vector), Some(format!("title != '{}'", title.replace("'", "''")))).await
                } else {
                    let vector = self.embedder.embed(query_str)?;
                    self.store.hybrid_search(Some(vector), None).await
                }
            }
            SearchTool::Metadata { filters } => {
                let filter_str = self.build_filter_string(&filters);
                self.store.hybrid_search(None, filter_str).await
            }
            SearchTool::Hybrid { query, filters } => {
                let vector = self.embedder.embed(&query)?;
                let filter_str = self.build_filter_string(&filters);
                self.store.hybrid_search(Some(vector), filter_str).await
            }
        }
    }

    fn build_filter_string(&self, filters: &SearchFilters) -> Option<String> {
        let mut parts = Vec::new();
        if let Some(d) = &filters.director {
            parts.push(format!("director = '{}'", d.replace("'", "''")));
        }
        if let Some(c) = &filters.cast {
            parts.push(format!("cast CONTAINS '{}'", c.replace("'", "''")));
        }
        if let Some(y) = filters.year {
            parts.push(format!("year = {}", y));
        }
        if let Some(k) = &filters.keyword {
            parts.push(format!("keywords CONTAINS '{}'", k.replace("'", "''")));
        }
        
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" AND "))
        }
    }

    async fn select_tool(&self, query: &str) -> Result<SearchTool> {
        let prompt = format!(
            "You are a movie search assistant. Choose the correct search tool for the user query: '{}'.

            Available Tools:
            - semantic: Use for general plot, theme, or vibe descriptions.
              Example: {{\"tool\": \"semantic\", \"params\": {{\"query\": \"a mind-bending sci-fi\"}}}}
            
            - similarity: Use when the user asks for movies 'like' or 'similar to' a specific movie title.
              Example: {{\"tool\": \"similarity\", \"params\": {{\"title\": \"Inception\"}}}}
            
            - metadata: Use when the user searches for specific directors, cast members, years, or keywords without a plot description.
              Example: {{\"tool\": \"metadata\", \"params\": {{\"filters\": {{\"director\": \"Christopher Nolan\"}}}}}}
            
            - hybrid: Use when the user combines a plot/theme description with specific metadata like a director or year.
              Example: {{\"tool\": \"hybrid\", \"params\": {{\"query\": \"space exploration\", \"filters\": {{\"director\": \"Christopher Nolan\"}}}}}}

            Rules:
            1. If the query is 'movies like [Title]', use 'similarity'.
            2. If the query mentions a director (e.g., 'by [Name]' or '[Name] movies'), include that in 'filters.director'.
            3. If the query mentions a year (e.g., 'from 1994' or 'in 1994'), include that in 'filters.year'.
            4. If the query mentions an actor/cast (e.g., 'with Tom Hanks' or 'starring Tom Hanks'), include that in 'filters.cast'.
            5. Return ONLY valid JSON that matches the SearchTool schema.",
            query
        );

        let request = GenerationRequest::new(self.model.clone(), prompt)
            .format(FormatType::Json);

        match self.ollama.generate(request).await {
            Ok(res) => {
                match serde_json::from_str::<SearchTool>(&res.response) {
                    Ok(tool) => Ok(tool),
                    Err(_) => {
                        Ok(SearchTool::Semantic { query: query.to_string() })
                    }
                }
            }
            Err(_) => {
                Ok(SearchTool::Semantic { query: query.to_string() })
            }
        }
    }
}
