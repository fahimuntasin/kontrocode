use std::pin::Pin;
use std::sync::LazyLock;

use async_trait::async_trait;
use futures::Stream;
use kontrocode_core::{
    CompletionRequest, CompletionResponse, FinishReason, Message, ModelId, ModelSpec, Result,
    StreamChunk, Usage,
};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use tracing::warn;

use crate::provider::Provider;

struct ProviderConfig {
    id: &'static str,
    api_url: &'static str,
    api_key_env: &'static str,
    header_name: &'static str,
    header_value_prefix: &'static str,
    models_spec: Vec<ModelSpec>,
    model_map: fn(&str) -> String,
}

fn identity_model(name: &str) -> String {
    name.to_string()
}

pub struct GenericProvider {
    client: Client,
    api_key: String,
    config: ProviderConfig,
}

impl GenericProvider {
    pub fn new(config: ProviderConfig, api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            config,
        }
    }

    pub fn from_env(config: ProviderConfig) -> Option<Self> {
        std::env::var(config.api_key_env)
            .ok()
            .map(|key| Self::new(config, key))
    }
}

#[async_trait]
impl Provider for GenericProvider {
    fn id(&self) -> &'static str {
        self.config.id
    }

    fn models(&self) -> &[ModelSpec] {
        &self.config.models_spec
    }

    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        let model_name = (self.config.model_map)(req.model.model());
        let messages: Vec<_> = req
            .messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    kontrocode_core::message::Role::System => "system",
                    kontrocode_core::message::Role::Assistant => "assistant",
                    _ => "user",
                };
                json!({"role": role, "content": m.content})
            })
            .collect();

        let body = json!({
            "model": model_name,
            "messages": messages,
            "max_tokens": req.max_tokens,
        });

        let auth_value = format!("{}{}", self.config.header_value_prefix, self.api_key);
        let resp = self
            .client
            .post(self.config.api_url)
            .header(self.config.header_name, &auth_value)
            .json(&body)
            .send()
            .await
            .map_err(|e| kontrocode_core::Error::provider(self.config.id, e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            warn!(provider = self.config.id, "{text}");
            return Err(kontrocode_core::Error::provider(self.config.id, text));
        }

        let resp: OpenAiCompatResp = resp.json().await.map_err(|e| {
            kontrocode_core::Error::provider(self.config.id, e.to_string())
        })?;

        let choice = resp.choices.first().ok_or_else(|| {
            kontrocode_core::Error::provider(self.config.id, "empty choices")
        })?;

        Ok(CompletionResponse {
            model: req.model,
            message: Message::assistant(choice.message.content.clone()),
            finish_reason: match choice.finish_reason.as_deref() {
                Some("stop") => FinishReason::Stop,
                Some("length") => FinishReason::Length,
                Some("tool_calls") => FinishReason::ToolCalls,
                _ => FinishReason::Stop,
            },
            usage: Usage {
                input_tokens: resp.usage.input_tokens,
                output_tokens: resp.usage.output_tokens,
                cost_usd: 0.0,
            },
        })
    }

    async fn stream(
        &self,
        _req: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        Err(kontrocode_core::Error::provider(
            self.config.id,
            "streaming not yet implemented",
        ))
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiCompatResp {
    choices: Vec<OpenAiCompatChoice>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct OpenAiCompatChoice {
    message: OpenAiCompatMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiCompatMessage {
    content: String,
}

pub fn openai_models() -> Vec<ModelSpec> {
    vec![
        ModelSpec {
            id: ModelId::new("openai", "gpt-4o"),
            display_name: "GPT-4o".into(),
            context_window: 128_000,
            max_output_tokens: 16_384,
            input_price_per_mtok: 2.5,
            output_price_per_mtok: 10.0,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
        },
        ModelSpec {
            id: ModelId::new("openai", "gpt-4o-mini"),
            display_name: "GPT-4o Mini".into(),
            context_window: 128_000,
            max_output_tokens: 16_384,
            input_price_per_mtok: 0.15,
            output_price_per_mtok: 0.6,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
        },
        ModelSpec {
            id: ModelId::new("openai", "o3-mini"),
            display_name: "o3-mini".into(),
            context_window: 200_000,
            max_output_tokens: 100_000,
            input_price_per_mtok: 1.1,
            output_price_per_mtok: 4.4,
            supports_tools: false,
            supports_streaming: false,
            supports_vision: false,
        },
    ]
}

pub fn deepseek_models() -> Vec<ModelSpec> {
    vec![
        ModelSpec {
            id: ModelId::new("deepseek", "v3"),
            display_name: "DeepSeek V3".into(),
            context_window: 64_000,
            max_output_tokens: 8_192,
            input_price_per_mtok: 0.27,
            output_price_per_mtok: 1.1,
            supports_tools: true,
            supports_streaming: false,
            supports_vision: false,
        },
        ModelSpec {
            id: ModelId::new("deepseek", "r1"),
            display_name: "DeepSeek R1".into(),
            context_window: 64_000,
            max_output_tokens: 8_192,
            input_price_per_mtok: 0.55,
            output_price_per_mtok: 2.19,
            supports_tools: true,
            supports_streaming: false,
            supports_vision: false,
        },
    ]
}

fn deepseek_model_map(name: &str) -> String {
    if name == "r1" {
        "deepseek-reasoner".into()
    } else {
        "deepseek-chat".into()
    }
}

pub fn groq_models() -> Vec<ModelSpec> {
    vec![
        ModelSpec {
            id: ModelId::new("groq", "llama-3.3-70b"),
            display_name: "Llama 3.3 70B".into(),
            context_window: 128_000,
            max_output_tokens: 4_096,
            input_price_per_mtok: 0.59,
            output_price_per_mtok: 0.79,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: false,
        },
        ModelSpec {
            id: ModelId::new("groq", "mixtral-8x22b"),
            display_name: "Mixtral 8x22B".into(),
            context_window: 65_536,
            max_output_tokens: 4_096,
            input_price_per_mtok: 0.2,
            output_price_per_mtok: 0.2,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: false,
        },
    ]
}

pub fn google_models() -> Vec<ModelSpec> {
    vec![
        ModelSpec {
            id: ModelId::new("google", "gemini-2.5-flash"),
            display_name: "Gemini 2.5 Flash".into(),
            context_window: 1_048_576,
            max_output_tokens: 8_192,
            input_price_per_mtok: 0.15,
            output_price_per_mtok: 0.6,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
        },
        ModelSpec {
            id: ModelId::new("google", "gemini-2.5-pro"),
            display_name: "Gemini 2.5 Pro".into(),
            context_window: 2_097_152,
            max_output_tokens: 16_384,
            input_price_per_mtok: 1.25,
            output_price_per_mtok: 5.0,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
        },
    ]
}

pub fn xai_models() -> Vec<ModelSpec> {
    vec![
        ModelSpec {
            id: ModelId::new("xai", "grok-3"),
            display_name: "Grok 3".into(),
            context_window: 1_000_000,
            max_output_tokens: 16_384,
            input_price_per_mtok: 3.0,
            output_price_per_mtok: 15.0,
            supports_tools: true,
            supports_streaming: false,
            supports_vision: true,
        },
        ModelSpec {
            id: ModelId::new("xai", "grok-3-mini"),
            display_name: "Grok 3 Mini".into(),
            context_window: 1_000_000,
            max_output_tokens: 16_384,
            input_price_per_mtok: 0.3,
            output_price_per_mtok: 0.5,
            supports_tools: true,
            supports_streaming: false,
            supports_vision: true,
        },
    ]
}

pub fn mistral_models() -> Vec<ModelSpec> {
    vec![
        ModelSpec {
            id: ModelId::new("mistral", "mistral-large"),
            display_name: "Mistral Large".into(),
            context_window: 128_000,
            max_output_tokens: 16_384,
            input_price_per_mtok: 2.0,
            output_price_per_mtok: 6.0,
            supports_tools: true,
            supports_streaming: false,
            supports_vision: false,
        },
        ModelSpec {
            id: ModelId::new("mistral", "codestral"),
            display_name: "Codestral".into(),
            context_window: 256_000,
            max_output_tokens: 16_384,
            input_price_per_mtok: 1.0,
            output_price_per_mtok: 3.0,
            supports_tools: true,
            supports_streaming: false,
            supports_vision: false,
        },
    ]
}

pub fn anthropic_models() -> Vec<ModelSpec> {
    vec![
        ModelSpec {
            id: ModelId::new("anthropic", "claude-haiku-3-5"),
            display_name: "Claude Haiku 3.5".into(),
            context_window: 200_000,
            max_output_tokens: 4_096,
            input_price_per_mtok: 0.8,
            output_price_per_mtok: 4.0,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
        },
        ModelSpec {
            id: ModelId::new("anthropic", "claude-sonnet-4"),
            display_name: "Claude Sonnet 4".into(),
            context_window: 200_000,
            max_output_tokens: 8_192,
            input_price_per_mtok: 3.0,
            output_price_per_mtok: 15.0,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
        },
        ModelSpec {
            id: ModelId::new("anthropic", "claude-opus-4-5"),
            display_name: "Claude Opus 4.5".into(),
            context_window: 200_000,
            max_output_tokens: 16_384,
            input_price_per_mtok: 15.0,
            output_price_per_mtok: 75.0,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
        },
    ]
}

pub fn create_openai() -> Option<GenericProvider> {
    GenericProvider::from_env(ProviderConfig {
        id: "openai",
        api_url: "https://api.openai.com/v1/chat/completions",
        api_key_env: "OPENAI_API_KEY",
        header_name: "Authorization",
        header_value_prefix: "Bearer ",
        models_spec: openai_models(),
        model_map: identity_model,
    })
}

pub fn create_deepseek() -> Option<GenericProvider> {
    GenericProvider::from_env(ProviderConfig {
        id: "deepseek",
        api_url: "https://api.deepseek.com/v1/chat/completions",
        api_key_env: "DEEPSEEK_API_KEY",
        header_name: "Authorization",
        header_value_prefix: "Bearer ",
        models_spec: deepseek_models(),
        model_map: deepseek_model_map,
    })
}

pub fn create_groq() -> Option<GenericProvider> {
    GenericProvider::from_env(ProviderConfig {
        id: "groq",
        api_url: "https://api.groq.com/openai/v1/chat/completions",
        api_key_env: "GROQ_API_KEY",
        header_name: "Authorization",
        header_value_prefix: "Bearer ",
        models_spec: groq_models(),
        model_map: identity_model,
    })
}

pub fn create_google() -> Option<GenericProvider> {
    GenericProvider::from_env(ProviderConfig {
        id: "google",
        api_url: "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
        api_key_env: "GOOGLE_API_KEY",
        header_name: "x-goog-api-key",
        header_value_prefix: "",
        models_spec: google_models(),
        model_map: identity_model,
    })
}

pub fn create_xai() -> Option<GenericProvider> {
    GenericProvider::from_env(ProviderConfig {
        id: "xai",
        api_url: "https://api.x.ai/v1/chat/completions",
        api_key_env: "XAI_API_KEY",
        header_name: "Authorization",
        header_value_prefix: "Bearer ",
        models_spec: xai_models(),
        model_map: identity_model,
    })
}

pub fn create_mistral() -> Option<GenericProvider> {
    GenericProvider::from_env(ProviderConfig {
        id: "mistral",
        api_url: "https://api.mistral.ai/v1/chat/completions",
        api_key_env: "MISTRAL_API_KEY",
        header_name: "Authorization",
        header_value_prefix: "Bearer ",
        models_spec: mistral_models(),
        model_map: identity_model,
    })
}

pub fn create_anthropic() -> Option<GenericProvider> {
    GenericProvider::from_env(ProviderConfig {
        id: "anthropic",
        api_url: "https://api.anthropic.com/v1/messages",
        api_key_env: "ANTHROPIC_API_KEY",
        header_name: "x-api-key",
        header_value_prefix: "",
        models_spec: anthropic_models(),
        model_map: identity_model,
    })
}

pub fn together_models() -> Vec<ModelSpec> {
    vec![
        ModelSpec { id: ModelId::new("together", "llama-3.1-405b"), display_name: "Llama 3.1 405B".into(), context_window: 128_000, max_output_tokens: 4_096, input_price_per_mtok: 2.5, output_price_per_mtok: 2.5, supports_tools: true, supports_streaming: true, supports_vision: false },
        ModelSpec { id: ModelId::new("together", "mixtral-8x22b"), display_name: "Mixtral 8x22B".into(), context_window: 65_536, max_output_tokens: 4_096, input_price_per_mtok: 0.9, output_price_per_mtok: 0.9, supports_tools: true, supports_streaming: true, supports_vision: false },
    ]
}


pub fn fireworks_models() -> Vec<ModelSpec> {
    vec![
        ModelSpec { id: ModelId::new("fireworks", "llama-v3p1-70b"), display_name: "Llama 3.1 70B".into(), context_window: 128_000, max_output_tokens: 4_096, input_price_per_mtok: 0.9, output_price_per_mtok: 0.9, supports_tools: true, supports_streaming: true, supports_vision: false },
        ModelSpec { id: ModelId::new("fireworks", "mixtral-8x7b"), display_name: "Mixtral 8x7B".into(), context_window: 32_768, max_output_tokens: 4_096, input_price_per_mtok: 0.5, output_price_per_mtok: 0.5, supports_tools: true, supports_streaming: true, supports_vision: false },
    ]
}


pub fn create_together() -> Option<GenericProvider> {
    GenericProvider::from_env(ProviderConfig {
        id: "together", api_url: "https://api.together.xyz/v1/chat/completions",
        api_key_env: "TOGETHER_API_KEY", header_name: "Authorization",
        header_value_prefix: "Bearer ", models_spec: together_models(), model_map: identity_model,
    })
}

pub fn create_fireworks() -> Option<GenericProvider> {
    GenericProvider::from_env(ProviderConfig {
        id: "fireworks", api_url: "https://api.fireworks.ai/inference/v1/chat/completions",
        api_key_env: "FIREWORKS_API_KEY", header_name: "Authorization",
        header_value_prefix: "Bearer ", models_spec: fireworks_models(), model_map: identity_model,
    })
}
