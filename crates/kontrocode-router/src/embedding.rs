use kontrocode_core::embedding::Embedding;
use kontrocode_core::Result;
use reqwest::Client;
use serde::Deserialize;
use tracing::warn;

const OPENAI_EMBED_URL: &str = "https://api.openai.com/v1/embeddings";

pub struct EmbeddingClient {
    client: Client,
    api_key: String,
    model: String,
}

impl EmbeddingClient {
    pub fn new(api_key: String, model: &str) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: model.to_string(),
        }
    }

    pub fn from_env() -> Option<Self> {
        std::env::var("OPENAI_API_KEY").ok().map(|key| {
            let model = std::env::var("KONTROCODE_EMBED_MODEL")
                .unwrap_or_else(|_| "text-embedding-3-small".into());
            Self::new(key, &model)
        })
    }

    pub async fn embed(&self, text: &str) -> Result<Embedding> {
        let body = serde_json::json!({
            "model": self.model,
            "input": text,
        });

        let resp = self
            .client
            .post(OPENAI_EMBED_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| kontrocode_core::Error::other(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            warn!("embedding error: {text}");
            return Err(kontrocode_core::Error::other(text));
        }

        let resp: EmbedResponse = resp
            .json()
            .await
            .map_err(|e| kontrocode_core::Error::other(e.to_string()))?;

        let vector = resp
            .data
            .first()
            .map(|d| d.embedding.clone())
            .unwrap_or_default();

        Ok(Embedding {
            model: self.model.clone(),
            dimensions: vector.len(),
            vector,
        })
    }
}

#[derive(Debug, Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedData>,
}

#[derive(Debug, Deserialize)]
struct EmbedData {
    embedding: Vec<f32>,
}
