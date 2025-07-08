use color_eyre::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct CompletionResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: Message,
}

#[derive(Clone, Debug)]
pub struct InferenceEngine {
    client: Client,
    api_key: String,
    base_url: String,
}

impl InferenceEngine {
    pub fn new() -> Result<Self> {
        let api_key = env::var("OPENROUTER_API_KEY").map_err(|_| {
            color_eyre::eyre::eyre!("OPENROUTER_API_KEY environment variable not set")
        })?;

        let client = Client::new();
        let base_url = "https://openrouter.ai/api/v1".to_string();
        //let base_url = "http://localhost:8888/v1".to_string();

        Ok(Self {
            client,
            api_key,
            base_url,
        })
    }

    pub async fn completion(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(color_eyre::eyre::eyre!(
                "API request failed with status {}: {}",
                status,
                text
            ));
        }

        let completion_response: CompletionResponse = response.json().await?;
        Ok(completion_response)
    }

    pub async fn imagine_command(&self, request: String) -> Result<CompletionResponse> {
        let request = CompletionRequest {
            model: "google/gemini-2.5-flash".to_string(),
            messages: vec![
                Message {
                    role: "system".into(),
                    content: "You are system designed to emit bash commands, fulfilling the user's request. To achieve your goal, emit a single line command and only that command to achieve the user's request. When possible, use verbose command switches, to convey intent. You can safely assume whatever programs needed to achieve your goal are avaiable to you, such as jq ffmpeg, etc. When emitting your command, emit only the command, with no markdown formatting".into()
                },
                Message {
                    role: "user".to_string(),
                    content: request,
                }
            ],
            max_tokens: Some(1000),
            temperature: Some(0.7),
        };

        self.completion(request).await
    }

    pub async fn inspect_command(&self, request: String) -> Result<CompletionResponse> {
        let request = CompletionRequest {
            model: "google/gemini-2.5-flash".to_string(),
            messages: vec![
                Message {
                    role: "system".into(),
                    content: "The user is going to pass in a command. Your role is to inspect this for safety, evaluating whether or not the command could cause unexpected harm. Unexpected harm may be deleting or removing more files than intended. Your response should first lead with a Y for safe or N for unsafe. Your analysis should be concise, focussing on any caveats first and foremost".into()
                },
                Message {
                    role: "user".to_string(),
                    content: request,
                }
            ],
            max_tokens: Some(1000),
            temperature: Some(0.7),
        };

        self.completion(request).await
    }
}
