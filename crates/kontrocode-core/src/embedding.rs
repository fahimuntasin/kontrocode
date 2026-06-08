use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub model: String,
    pub vector: Vec<f32>,
    pub dimensions: usize,
}

impl Embedding {
    pub fn cosine_similarity(&self, other: &Embedding) -> f64 {
        if self.vector.len() != other.vector.len() {
            return 0.0;
        }
        let dot: f64 = self
            .vector
            .iter()
            .zip(other.vector.iter())
            .map(|(a, b)| (*a as f64) * (*b as f64))
            .sum();
        let norm_a: f64 = self.vector.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
        let norm_b: f64 = other.vector.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot / (norm_a * norm_b)
    }

    pub fn zero(dimensions: usize) -> Self {
        Self {
            model: "none".into(),
            vector: vec![0.0; dimensions],
            dimensions,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedFact {
    pub fact_id: String,
    pub text: String,
    pub confidence: f64,
    pub embedding: Embedding,
}
