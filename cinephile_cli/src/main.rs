use anyhow::{Result, Context};
use clap::{Parser, Subcommand};
use cinephile_core::{Store, Embedder, Enricher, Searcher, scan_directories, Movie};
use dotenvy::dotenv;
use std::path::PathBuf;
use tabled::{Table, Tabled};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use directories::ProjectDirs;
use serde::{Serialize, Deserialize};
use futures_util::StreamExt;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Configure settings
    Config {
        #[arg(long)]
        set_tmdb_key: Option<String>,
        #[arg(long)]
        set_ollama_model: Option<String>,
    },
    /// Add a directory to the watch list
    Add {
        path: PathBuf,
    },
    /// Index all movies in the watched directories
    Index,
    /// Search for movies
    Search {
        query: String,
        #[arg(long)]
        play: bool,
    },
}

#[derive(Serialize, Deserialize)]
struct AppConfig {
    tmdb_api_key: Option<String>,
    #[serde(default = "default_ollama_model")]
    ollama_model: String,
    watched_directories: Vec<PathBuf>,
}

fn default_ollama_model() -> String {
    "llama3.2".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            tmdb_api_key: None,
            ollama_model: default_ollama_model(),
            watched_directories: vec![],
        }
    }
}

#[derive(Tabled)]
struct SearchResult {
    title: String,
    year: String,
    path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let cli = Cli::parse();

    let proj_dirs = ProjectDirs::from("com", "cinephile", "cinephile")
        .context("Failed to get project directories")?;
    let config_dir = proj_dirs.config_dir();
    fs::create_dir_all(config_dir)?;
    let config_path = config_dir.join("config.json");

    let mut config: AppConfig = if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        AppConfig::default()
    };

    match cli.command {
        Commands::Config { set_tmdb_key, set_ollama_model } => {
            if let Some(key) = set_tmdb_key {
                config.tmdb_api_key = Some(key);
                println!("TMDB API key updated.");
            }
            if let Some(model) = set_ollama_model {
                config.ollama_model = model;
                println!("Ollama model updated to: {}", config.ollama_model);
            }
            fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
        }
        Commands::Add { path } => {
            let abs_path = fs::canonicalize(path)?;
            if !config.watched_directories.contains(&abs_path) {
                config.watched_directories.push(abs_path);
                fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
                println!("Added directory: {:?}", config.watched_directories.last().unwrap());
            } else {
                println!("Directory already in watch list.");
            }
        }
        Commands::Index => {
            if config.tmdb_api_key.is_none() {
                anyhow::bail!("TMDB API key not set. Use 'cinephile config --set-tmdb-key <KEY>'");
            }
            std::env::set_var("TMDB_API_KEY", config.tmdb_api_key.as_ref().unwrap());

            let store = Store::new().await?;
            let mut embedder = Embedder::new().await?;
            let mut enricher = Enricher::new()?;
            enricher.set_model(config.ollama_model.clone());

            println!("Scanning directories...");
            let mut file_stream = scan_directories(config.watched_directories);
            let mut files: Vec<PathBuf> = Vec::new();
            while let Some(path) = file_stream.next().await {
                files.push(path);
            }
            println!("Found {} files to index.", files.len());

            let pb = ProgressBar::new(files.len() as u64);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")?
                .progress_chars("#>-"));

            for path in files {
                let filename = path.file_name().unwrap().to_str().unwrap().to_string();
                pb.set_message(format!("Processing {}", filename));
                
                let movie = Movie::new(path.to_str().unwrap().to_string(), filename.clone(), "".to_string());
                
                match enricher.enrich(movie).await {
                    Ok(mut enriched) => {
                        if enriched.title.is_empty() {
                            pb.println(format!("Warning: Could not enrich metadata for {}", filename));
                        }
                        let text_to_embed = format!("{} {}", enriched.title, enriched.plot);
                        match embedder.embed(&text_to_embed) {
                            Ok(vector) => {
                                enriched.vector = vector;
                                if let Err(e) = store.upsert_movie(enriched).await {
                                    pb.println(format!("Error storing {}: {}", filename, e));
                                } else {
                                    pb.println(format!("Successfully indexed: {}", filename));
                                }
                            }
                            Err(e) => pb.println(format!("Error embedding {}: {}", filename, e)),
                        }
                    }
                    Err(e) => pb.println(format!("Error enriching {}: {}", filename, e)),
                }
                pb.inc(1);
            }
            pb.finish_with_message("Indexing complete.");
        }
        Commands::Search { query, play } => {
            if config.tmdb_api_key.is_none() {
                anyhow::bail!("TMDB API key not set. Use 'cinephile config --set-tmdb-key <KEY>'");
            }
            std::env::set_var("TMDB_API_KEY", config.tmdb_api_key.as_ref().unwrap());

            let store = Store::new().await?;
            let embedder = Embedder::new().await?;
            let mut searcher = Searcher::new(store, embedder);
            searcher.set_model(config.ollama_model.clone());

            let results = searcher.search(&query).await?;
            println!("Found {} results.", results.len());
            
            if results.is_empty() {
                println!("No results found.");
                return Ok(());
            }

            let table_data: Vec<SearchResult> = results.iter().map(|m| SearchResult {
                title: m.title.clone(),
                year: m.year.map(|y| y.to_string()).unwrap_or_default(),
                path: m.filepath.clone(),
            }).collect();

            println!("{}", Table::new(table_data));

            if play {
                let top_result = &results[0];
                println!("Playing: {}", top_result.title);
                opener::open(&top_result.filepath)?;
            }
        }
    }

    Ok(())
}
