use serde::{Deserialize, Serialize};
use std::error::Error as _;
use std::time::Duration;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiProvider {
    Ollama,
    OpenAi,
}

impl AiProvider {
    pub fn from_label(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "openai" => AiProvider::OpenAi,
            _ => AiProvider::Ollama,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AiProvider::Ollama => "ollama",
            AiProvider::OpenAi => "openai",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AiConfig {
    pub provider: AiProvider,
    pub url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: f64,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub raw_response: String,
    pub parsed_name: Option<String>,
    pub parsed_theme: Option<String>,
    pub parsed_desc: Option<String>,
    pub parsed_tags: Option<String>,
    pub parsed_colors: Option<String>,
}

pub struct AiClient {
    config: AiConfig,
    http: reqwest::Client,
}

/// Walk a `reqwest::Error`'s source chain so the user sees the real cause
/// (TimedOut / ConnectionReset / DNS / TLS / …) instead of the generic
/// "error sending request" wrapper. The `is_*` predicates also let us
/// produce a German hint for the most common failure modes.
fn describe_reqwest_error(e: &reqwest::Error) -> String {
    let mut parts: Vec<String> = vec![e.to_string()];
    let mut src: Option<&dyn std::error::Error> = e.source();
    while let Some(s) = src {
        parts.push(s.to_string());
        src = s.source();
    }
    let chain = parts.join(" → ");

    let hint = if e.is_timeout() {
        " (Zeitüberschreitung — bitte ai_timeout_ms in den Einstellungen erhöhen)"
    } else if e.is_connect() {
        " (Verbindung zur Ollama-Adresse fehlgeschlagen — IP, Port, Firewall prüfen)"
    } else if e.is_request() {
        " (Anfrage konnte nicht gesendet werden)"
    } else {
        ""
    };
    format!("{chain}{hint}")
}

impl AiClient {
    pub fn new(config: AiConfig) -> Result<Self, AppError> {
        // Audit-follow-up: split the timeout into a short connect_timeout
        // (so connection failures fail fast) and a longer overall
        // `timeout` (so vision inference has time to run). Vision models
        // routinely take 30–120 s on first call when Ollama is loading
        // the model, so we ensure the overall budget is at least 120 s
        // even if the user hasn't bumped `ai_timeout_ms`.
        const MIN_TIMEOUT_MS: u64 = 120_000;
        const CONNECT_TIMEOUT_MS: u64 = 10_000;
        let total_timeout_ms = config.timeout_ms.max(MIN_TIMEOUT_MS);
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(total_timeout_ms))
            .connect_timeout(Duration::from_millis(CONNECT_TIMEOUT_MS))
            .build()
            .map_err(|e| AppError::Ai(format!("HTTP-Client Fehler: {e}")))?;
        Ok(Self { config, http })
    }

    pub async fn analyze(&self, image_base64: &str, prompt: &str) -> Result<AiResponse, AppError> {
        let raw = match self.config.provider {
            AiProvider::Ollama => self.analyze_ollama(image_base64, prompt).await?,
            AiProvider::OpenAi => self.analyze_openai(image_base64, prompt).await?,
        };
        Ok(parse_ai_json(&raw))
    }

    pub async fn test_connection(&self) -> bool {
        match self.config.provider {
            AiProvider::Ollama => self.test_ollama().await,
            AiProvider::OpenAi => self.test_openai().await,
        }
    }

    async fn analyze_ollama(&self, image_base64: &str, prompt: &str) -> Result<String, AppError> {
        let url = format!("{}/api/generate", self.config.url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.config.model,
            "prompt": prompt,
            "images": [image_base64],
            "stream": false,
            "options": {
                "temperature": self.config.temperature,
            }
        });

        // Audit-follow-up: log what we actually send so failures can be
        // diagnosed without enabling Ollama-side tracing.
        log::info!(
            "Ollama analyze: POST {url} model={} image_b64_bytes={} prompt_chars={}",
            self.config.model,
            image_base64.len(),
            prompt.chars().count()
        );

        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Ai(format!(
                "Ollama-Anfrage an {url} fehlgeschlagen: {}",
                describe_reqwest_error(&e)
            )))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Ai(format!(
                "Ollama-Fehler {status}: {text}"
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::Ai(format!(
                "Ollama-Antwort ungueltig: {}",
                describe_reqwest_error(&e)
            )))?;

        json["response"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::Ai("Ollama: Kein 'response'-Feld in Antwort".into()))
    }

    async fn analyze_openai(&self, image_base64: &str, prompt: &str) -> Result<String, AppError> {
        let url = format!(
            "{}/v1/chat/completions",
            self.config.url.trim_end_matches('/')
        );

        let body = serde_json::json!({
            "model": self.config.model,
            "temperature": self.config.temperature,
            "max_tokens": 2048,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": prompt
                        },
                        {
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:image/png;base64,{image_base64}")
                            }
                        }
                    ]
                }
            ]
        });

        let mut req = self.http.post(&url).json(&body);
        if let Some(ref key) = self.config.api_key {
            req = req.bearer_auth(key);
        }

        log::info!(
            "OpenAI analyze: POST {url} model={} image_b64_bytes={} prompt_chars={}",
            self.config.model,
            image_base64.len(),
            prompt.chars().count()
        );

        let resp = req
            .send()
            .await
            .map_err(|e| AppError::Ai(format!(
                "OpenAI-Anfrage an {url} fehlgeschlagen: {}",
                describe_reqwest_error(&e)
            )))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Ai(format!("OpenAI-Fehler {status}: {text}")));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::Ai(format!(
                "OpenAI-Antwort ungueltig: {}",
                describe_reqwest_error(&e)
            )))?;

        json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::Ai("OpenAI: Kein content in Antwort".into()))
    }

    async fn test_ollama(&self) -> bool {
        let url = format!("{}/api/tags", self.config.url.trim_end_matches('/'));
        match self.http.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    async fn test_openai(&self) -> bool {
        let url = format!("{}/v1/models", self.config.url.trim_end_matches('/'));
        let mut req = self.http.get(&url);
        if let Some(ref key) = self.config.api_key {
            req = req.bearer_auth(key);
        }
        match req.send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

/// Parse LLM response text as JSON, extracting structured fields.
/// Handles markdown code fences (```json ... ```) around the JSON.
fn parse_ai_json(raw: &str) -> AiResponse {
    let trimmed = raw.trim();

    // Try to extract JSON from markdown code fences
    let json_str = if let Some(start) = trimmed.find("```json") {
        let after = &trimmed[start + 7..];
        if let Some(end) = after.find("```") {
            after[..end].trim()
        } else {
            after.trim()
        }
    } else if let Some(start) = trimmed.find("```") {
        let after = &trimmed[start + 3..];
        if let Some(end) = after.find("```") {
            after[..end].trim()
        } else {
            after.trim()
        }
    } else {
        trimmed
    };

    // Find the first { and last } to handle surrounding text
    let json_obj = if let (Some(start), Some(end)) = (json_str.find('{'), json_str.rfind('}')) {
        &json_str[start..=end]
    } else {
        return AiResponse {
            raw_response: raw.to_string(),
            parsed_name: None,
            parsed_theme: None,
            parsed_desc: None,
            parsed_tags: None,
            parsed_colors: None,
        };
    };

    match serde_json::from_str::<serde_json::Value>(json_obj) {
        Ok(val) => AiResponse {
            raw_response: raw.to_string(),
            parsed_name: val.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            parsed_theme: val.get("theme").and_then(|v| v.as_str()).map(|s| s.to_string()),
            parsed_desc: val
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            parsed_tags: val.get("tags").map(|v| v.to_string()),
            parsed_colors: val.get("colors").map(|v| v.to_string()),
        },
        Err(_) => AiResponse {
            raw_response: raw.to_string(),
            parsed_name: None,
            parsed_theme: None,
            parsed_desc: None,
            parsed_tags: None,
            parsed_colors: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ai_json_plain() {
        let raw = r##"{"name": "Rose", "theme": "Floral", "description": "A rose design", "tags": ["floral", "nature"], "colors": [{"hex": "#FF0000", "name": "Red"}]}"##;
        let resp = parse_ai_json(raw);
        assert_eq!(resp.parsed_name.as_deref(), Some("Rose"));
        assert_eq!(resp.parsed_theme.as_deref(), Some("Floral"));
        assert_eq!(resp.parsed_desc.as_deref(), Some("A rose design"));
        assert!(resp.parsed_tags.is_some());
        assert!(resp.parsed_colors.is_some());
    }

    #[test]
    fn test_parse_ai_json_with_code_fence() {
        let raw = "Here is the analysis:\n```json\n{\"name\": \"Star\", \"theme\": \"Geometric\"}\n```\n";
        let resp = parse_ai_json(raw);
        assert_eq!(resp.parsed_name.as_deref(), Some("Star"));
        assert_eq!(resp.parsed_theme.as_deref(), Some("Geometric"));
    }

    #[test]
    fn test_parse_ai_json_invalid() {
        let raw = "This is not JSON at all";
        let resp = parse_ai_json(raw);
        assert!(resp.parsed_name.is_none());
        assert_eq!(resp.raw_response, raw);
    }

    #[test]
    fn test_ai_provider_from_str() {
        assert!(matches!(AiProvider::from_label("ollama"), AiProvider::Ollama));
        assert!(matches!(AiProvider::from_label("OpenAI"), AiProvider::OpenAi));
        assert!(matches!(AiProvider::from_label("openai"), AiProvider::OpenAi));
        assert!(matches!(AiProvider::from_label("unknown"), AiProvider::Ollama));
    }
}
