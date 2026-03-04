use anyhow::{Result, Context};
use crate::models::Movie;
use ollama_rs::{
    Ollama, 
    generation::completion::request::GenerationRequest,
    generation::parameters::FormatType
};
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use std::env;

pub struct Enricher {
    tmdb_api_key: String,
    ollama: Ollama,
    http_client: Client,
    model: String,
}

#[derive(Deserialize)]
struct TmdbSearchResponse {
    results: Vec<TmdbMovie>,
}

#[derive(Deserialize)]
struct TmdbMovie {
    id: u32,
    title: String,
    release_date: Option<String>,
    overview: String,
}

#[derive(Deserialize)]
struct TmdbCredits {
    cast: Vec<TmdbCast>,
    crew: Vec<TmdbCrew>,
}

#[derive(Deserialize)]
struct TmdbCast {
    name: String,
}

#[derive(Deserialize)]
struct TmdbCrew {
    name: String,
    job: String,
}

impl Enricher {
    pub fn new() -> Result<Self> {
        let tmdb_api_key = env::var("TMDB_API_KEY")
            .context("TMDB_API_KEY not found in environment")?;
        
        Ok(Self {
            tmdb_api_key,
            ollama: Ollama::default(),
            http_client: Client::new(),
            model: "qwen3.5:0.8b".to_string(),
        })
    }

    pub fn set_model(&mut self, model: String) {
        self.model = model;
    }

    pub async fn enrich(&self, mut movie: Movie) -> Result<Movie> {
        // Step 1: Clean filename
        let (title, year) = self.clean_filename(&movie.filename).await?;
        movie.title = title;
        movie.year = year;

        // Step 2: Fetch TMDB data
        if let Some(tmdb_movie) = self.search_tmdb(&movie.title, movie.year).await? {
            movie.title = tmdb_movie.title;
            movie.plot = tmdb_movie.overview;
            
            if let Some(date) = tmdb_movie.release_date {
                if date.len() >= 4 {
                    if let Ok(y) = date[0..4].parse::<u16>() {
                        movie.year = Some(y);
                    }
                }
            }

            // Fetch credits
            let (director, cast) = self.fetch_credits(tmdb_movie.id).await?;
            movie.director = director;
            movie.cast = cast;
        }

        Ok(movie)
    }

    async fn clean_filename(&self, filename: &str) -> Result<(String, Option<u16>)> {
        // Simple regex: /(.*?)(\d{4})/
        let re = Regex::new(r"(?i)^(.*?)[\. \(\[]?(\d{4})[\. \)\]]?.*")?;
        if let Some(caps) = re.captures(filename) {
            let title = caps.get(1).unwrap().as_str().replace('.', " ").trim().to_string();
            let year = caps.get(2).unwrap().as_str().parse::<u16>().ok();
            return Ok((title, year));
        }

        // Fallback to Ollama with JSON mode
        let prompt = format!(
            "Analyze this film filename and extract the title and release year. Return a JSON object with 'title' (string) and 'year' (integer or null). Filename: '{}'",
            filename
        );
        
        let request = GenerationRequest::new(self.model.clone(), prompt)
            .format(FormatType::Json);
        
        match self.ollama.generate(request).await {
            Ok(res) => {
                let response_text = res.response;
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&response_text) {
                    let title = data["title"].as_str().unwrap_or(filename).to_string();
                    let year = data["year"].as_u64().map(|y| y as u16);
                    return Ok((title, year));
                }
            }
            Err(_) => {}
        }

        // Final fallback: just clean dots
        let title = filename.replace('.', " ").trim().to_string();
        Ok((title, None))
    }

    async fn search_tmdb(&self, title: &str, year: Option<u16>) -> Result<Option<TmdbMovie>> {
        let mut url = format!(
            "https://api.themoviedb.org/3/search/movie?api_key={}&query={}",
            self.tmdb_api_key,
            urlencoding::encode(title)
        );
        if let Some(y) = year {
            url.push_str(&format!("&year={}", y));
        }

        let resp = self.http_client.get(url).send().await?.json::<TmdbSearchResponse>().await?;
        Ok(resp.results.into_iter().next())
    }

    async fn fetch_credits(&self, tmdb_id: u32) -> Result<(Option<String>, Vec<String>)> {
        let url = format!(
            "https://api.themoviedb.org/3/movie/{}/credits?api_key={}",
            tmdb_id, self.tmdb_api_key
        );

        let resp = self.http_client.get(url).send().await?.json::<TmdbCredits>().await?;
        
        let director = resp.crew.into_iter()
            .find(|c| c.job == "Director")
            .map(|c| c.name);
        
        let cast = resp.cast.into_iter()
            .take(5)
            .map(|c| c.name)
            .collect();

        Ok((director, cast))
    }
}
