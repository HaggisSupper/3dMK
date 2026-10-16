use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::blocking::Client;
use serde_json::{json, Value};

use crate::{
    parse_vlm_json, ObjectClassification, PerceptionError, Result, VlmHook, VlmObjectRequest,
};

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleVlmConfig {
    pub endpoint: String,
    pub model: String,
    pub api_key: Option<String>,
    pub timeout_seconds: u64,
    pub request_json_object: bool,
}

impl Default for OpenAiCompatibleVlmConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:1234/v1/chat/completions".into(),
            model: "local-vlm".into(),
            api_key: None,
            timeout_seconds: 120,
            request_json_object: true,
        }
    }
}

pub struct OpenAiCompatibleVlm {
    config: OpenAiCompatibleVlmConfig,
    client: Client,
}

impl OpenAiCompatibleVlm {
    pub fn new(config: OpenAiCompatibleVlmConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds.max(1)))
            .build()
            .map_err(|e| PerceptionError::VlmRequest(e.to_string()))?;
        Ok(Self { config, client })
    }

    fn prompt(&self, request: &VlmObjectRequest) -> String {
        let candidates = if request.candidate_labels.is_empty() {
            "No closed label list was supplied. Use a concise concrete noun phrase.".to_owned()
        } else {
            format!(
                "Choose from these labels when one fits: {}.",
                request.candidate_labels.join(", ")
            )
        };
        format!(
            "Classify the isolated object. The transparent pixels are not part of the object. \
             Geometry and image statistics: {}. Local classifier result: label='{}', confidence={:.3}. \
             {} Return JSON only with keys: label (string), confidence (0..1 number), \
             alternatives (array of {{label, confidence}}), rationale (short string). \
             Do not infer geometry that is not visible and do not propose edits to the source scene.",
            request.geometry_summary.compact_summary(),
            request.local_classification.label,
            request.local_classification.confidence,
            candidates,
        )
    }
}

impl VlmHook for OpenAiCompatibleVlm {
    fn model_id(&self) -> &str {
        &self.config.model
    }

    fn classify(&self, request: &VlmObjectRequest) -> Result<ObjectClassification> {
        let image_uri = format!(
            "data:image/png;base64,{}",
            STANDARD.encode(&request.png_bytes)
        );
        let mut body = json!({
            "model": &self.config.model,
            "temperature": 0,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a conservative industrial 3D object classifier. Return strict JSON only."
                },
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": self.prompt(request)},
                        {"type": "image_url", "image_url": {"url": image_uri}}
                    ]
                }
            ]
        });
        if self.config.request_json_object {
            body["response_format"] = json!({"type": "json_object"});
        }

        let mut builder = self.client.post(&self.config.endpoint).json(&body);
        if let Some(key) = self.config.api_key.as_ref() {
            builder = builder.bearer_auth(key);
        }
        let response = builder
            .send()
            .map_err(|e| PerceptionError::VlmRequest(e.to_string()))?;
        let status = response.status();
        let payload: Value = response
            .json()
            .map_err(|e| PerceptionError::VlmResponse(e.to_string()))?;
        if !status.is_success() {
            return Err(PerceptionError::VlmRequest(format!(
                "HTTP {}: {}",
                status, payload
            )));
        }
        let content = payload
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                PerceptionError::VlmResponse(
                    "missing choices[0].message.content in OpenAI-compatible response".into(),
                )
            })?;
        Ok(parse_vlm_json(content)?.into_classification(self.model_id()))
    }
}
