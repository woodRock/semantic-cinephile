use anyhow::{Result, Context};
use ort::session::Session;
use ort::value::Value;
use ort::inputs;
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;
use directories::ProjectDirs;
use std::fs;
use std::io::copy;
use reqwest;

pub struct Embedder {
    session: Session,
    tokenizer: Tokenizer,
}

impl Embedder {
    pub async fn new() -> Result<Self> {
        let (model_path, tokenizer_path) = Self::ensure_model_files().await?;

        let session = Session::builder()?
            .commit_from_file(model_path)?;

        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;

        Ok(Self { session, tokenizer })
    }

    async fn ensure_model_files() -> Result<(PathBuf, PathBuf)> {
        let proj_dirs = ProjectDirs::from("com", "cinephile", "cinephile")
            .context("Failed to get project directories")?;
        let data_dir = proj_dirs.data_dir();
        fs::create_dir_all(data_dir)?;

        let model_path = data_dir.join("all-MiniLM-L6-v2.onnx");
        let tokenizer_path = data_dir.join("tokenizer.json");

        if !model_path.exists() {
            println!("Downloading embedding model...");
            let url = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx";
            Self::download_file(url, &model_path).await?;
        }

        if !tokenizer_path.exists() {
            println!("Downloading tokenizer...");
            let url = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json";
            Self::download_file(url, &tokenizer_path).await?;
        }

        Ok((model_path, tokenizer_path))
    }

    async fn download_file(url: &str, path: &Path) -> Result<()> {
        let response = reqwest::get(url).await?;
        let mut file = fs::File::create(path)?;
        let content = response.bytes().await?;
        copy(&mut content.as_ref(), &mut file)?;
        Ok(())
    }

    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        let encoding = self.tokenizer.encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenization error: {}", e))?;
        
        let ids = encoding.get_ids();
        let mask = encoding.get_attention_mask();
        let type_ids = encoding.get_type_ids();
        let seq_len = ids.len();

        let input_ids = Value::from_array(([1, seq_len], ids.iter().map(|&x| x as i64).collect::<Vec<_>>()))?;
        let attention_mask = Value::from_array(([1, seq_len], mask.iter().map(|&x| x as i64).collect::<Vec<_>>()))?;
        let token_type_ids = Value::from_array(([1, seq_len], type_ids.iter().map(|&x| x as i64).collect::<Vec<_>>()))?;

        let input_values = inputs![
            "input_ids" => input_ids,
            "attention_mask" => attention_mask,
            "token_type_ids" => token_type_ids
        ];

        let outputs = self.session.run(input_values)?;

        let output_value = outputs["last_hidden_state"].try_extract_tensor::<f32>()?;
        let (shape, data) = output_value;
        
        let dim = shape[2] as usize;
        
        let mut pooled = vec![0.0f32; dim];
        let mut count = 0.0f32;

        for i in 0..seq_len {
            if mask[i] != 0 {
                let offset = i * dim;
                for d in 0..dim {
                    pooled[d] += data[offset + d];
                }
                count += 1.0;
            }
        }

        if count > 0.0 {
            for d in 0..dim {
                pooled[d] /= count;
            }
        }

        let norm = pooled.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for d in 0..dim {
                pooled[d] /= norm;
            }
        }

        Ok(pooled)
    }
}
