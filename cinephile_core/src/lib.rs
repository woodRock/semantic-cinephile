pub mod models;
pub mod crawler;
pub mod enricher;
pub mod embedder;
pub mod store;
pub mod search;

pub use models::Movie;
pub use crawler::scan_directories;
pub use enricher::Enricher;
pub use embedder::Embedder;
pub use store::Store;
pub use search::Searcher;
