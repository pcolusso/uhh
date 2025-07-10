use std::collections::HashMap;
use std::pin::Pin;
use color_eyre::Result;
use reqwest::Client;
use serde::{de::{DeserializeOwned, IntoDeserializer}, Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
pub struct CompletionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<Message>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Stop>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // Advanced optional params...
    pub seed: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repetition_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<std::collections::HashMap<i32, f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_a: Option<f32>,

    // Reduce latency by providing the model with a predicted output ?
    // TODO: Mark not impl
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prediction: Option<Prediction>,

    // OpenRouter exclusive params
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transforms: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

// Doesn't seem to be a way to force OpenRouter to return only one choice..
impl Default for CompletionRequest {
    fn default() -> Self {
        Self {
            messages: Default::default(),
            prompt: Default::default(),
            model: Default::default(),
            response_format: Default::default(),
            stop: Default::default(),
            stream: Some(false), // Ceebs
            max_tokens: Default::default(),
            temperature: Default::default(),
            tools: Default::default(),
            tool_choice: Default::default(),
            seed: Default::default(),
            top_p: Default::default(),
            top_k: Default::default(),
            frequency_penalty: Default::default(),
            presence_penalty: Default::default(),
            repetition_penalty: Default::default(),
            logit_bias: Default::default(),
            top_logprobs: Default::default(),
            min_p: Default::default(),
            top_a: Default::default(),
            prediction: Default::default(),
            transforms: Default::default(),
            models: Default::default(),
            route: Default::default(),
            user: Default::default(),
        }
    }
}

impl CompletionResponse {
    pub fn assume_single_choice(mut self) -> Choice {
        if self.choices.len() != 1 {
            panic!("Expected exactly one choice, got {}", self.choices.len());
        }
        self.choices.pop().unwrap()
    }

    pub fn assume_single_text_choice(self) -> Result<String> {
        let choice = self.assume_single_choice();
        match choice {
            Choice::NonChat(_) => Err(color_eyre::eyre::eyre!("Non chat response")),
            Choice::Streaming(_) => Err(color_eyre::eyre::eyre!("Unexpected stream")),
            Choice::NonStreaming(r) => match r.message {
                Message::Assistant { content, .. } => match content {
                    Content::Text(text) => Ok(text),
                    Content::Parts(parts) => {
                        let text = parts
                            .iter()
                            .filter_map(|part| match part {
                                ContentPart::Text { text } => Some(text.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join(" ");
                        Ok(text)
                    }
                },
                _ => Err(color_eyre::eyre::eyre!("Expected assistant message")),
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    #[serde(rename = "user")]
    User {
        content: Content,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    #[serde(rename = "assistant")]
    Assistant {
        content: Content,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<ToolCall>>,
    },
    #[serde(rename = "system")]
    System {
        content: Content,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    #[serde(rename = "tool")]
    Tool {
        content: String,
        tool_call_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
}

impl Message {
    // Helper to rip out just the text of a message, regardless of kind.
    fn to_string(&self) -> String {
        let content = match self {
            Message::Assistant {
                content,
                name: _,
                tool_calls: _,
            } => content,
            Message::Tool {
                content,
                tool_call_id: _,
                name: _,
            } => {
                return content.clone();
            }
            Message::User { content, name: _ } => content,
            Message::System { content, name: _ } => content,
        };

        match content {
            Content::Text(text) => text.clone(),
            Content::Parts(parts) => parts
                .iter()
                .filter_map(|part| match part {
                    ContentPart::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompletionResponse {
    pub id: String,
    pub choices: Vec<Choice>,
    pub created: u64,
    pub model: String,
    pub object: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ResponseUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Choice {
    NonStreaming(NonStreamingChoice),
    Streaming(StreamingChoice),
    NonChat(NonChatChoice),
}

#[derive(Debug, Deserialize)]
pub struct NonStreamingChoice {
    pub message: Message,
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct StreamingChoice {
    pub delta: MessageDelta,
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct NonChatChoice {
    pub text: String,
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct MessageDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Deserialize)]
pub struct ResponseUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Stop {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prediction {
    #[serde(rename = "type")]
    pub prediction_type: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDescription,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionDescription {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // Can we just hammer these down to strings?
    pub parameters: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    String(String),
    Object {
        #[serde(rename = "type")]
        choice_type: String,
        function: FunctionChoice,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionChoice {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Clone, Debug)]
pub struct InferenceEngine {
    client: Client,
    api_key: String,
    base_url: String,
    model_ident: String,
    input: Option<String>,
    output: Option<String>,
}

type SyncToolFunc = fn(String) -> String;
type AsyncToolFunc = fn(String) -> Pin<Box<dyn Future<Output = String>>>;

enum ToolFunc {
    SyncTool(SyncToolFunc),
    AsyncTool(AsyncToolFunc)
}

struct ToolBelt {
    tools: HashMap<String, ToolFunc>,
    desc: HashMap<String, FunctionDescription>
}

impl ToolBelt {
    async fn call(
        &self,
        ToolCall {
            id,
            call_type,
            function,
        }: ToolCall,
    ) -> Message {
        match self.tools.get(&function.name) {
            Some(ToolFunc::SyncTool(f)) => {
                let res = f(function.arguments);
                Message::Tool { content: res, name: None, tool_call_id: "undef".into() }
            }
            Some(ToolFunc::AsyncTool(f)) => {
                let res = f(function.arguments).await;
                Message::Tool { content: res, tool_call_id: "undef".into(), name: None }
            }
            None => {
                Message::Tool { content: "Tool does not exist".into(), tool_call_id: "undef".into(), name: None }
            }
        }
    }
}

impl InferenceEngine {
    pub fn new(
        api_key: String,
        base_url: String,
        model_ident: String,
        input: Option<String>,
        output: Option<String>,
    ) -> Result<Self> {
        let client = Client::new();

        Ok(Self {
            client,
            api_key,
            input,
            output,
            model_ident,
            base_url,
        })
    }

    pub async fn tool_completion(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        loop {
            let result: Result<CompletionResponse> = self.completion(request.clone()).await;
        }
    }

    pub async fn completion<T: DeserializeOwned>(&self, request: CompletionRequest) -> Result<T> {
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

        let completion_response: T = response.json().await?;
        Ok(completion_response)
    }

    pub async fn imagine_command(&self, request: String) -> Result<String> {
        let mut base_prompt = "You are system designed to emit bash commands, fulfilling the user's request. To achieve your goal, emit a single line command and only that command to achieve the user's request. When possible, use verbose command switches, to convey intent. You can safely assume whatever programs needed to achieve your goal are avaiable to you, such as jq ffmpeg, etc. When emitting your command, emit only the command, with no markdown formatting\n".to_owned();

        if let Some(i) = self.input.as_ref() {
            base_prompt.push_str(&format!("An output path has been provided, it is {i}\n"));
        }

        if let Some(o) = self.output.as_ref() {
            base_prompt.push_str(&format!("An output path has been provided, it is {o}\n"));
        }

        let request = CompletionRequest {
            model: Some(self.model_ident.clone()),
            messages: Some(vec![
                Message::System {
                    content: Content::Text(base_prompt),
                    name: None,
                },
                Message::User {
                    content: Content::Text(request),
                    name: None,
                },
            ]),
            max_tokens: Some(1000),
            temperature: Some(0.7),
            ..Default::default()
        };

        // TODO: Asset final response is from Assistant
        let response: CompletionResponse = self.completion(request).await?;
        response.assume_single_text_choice()
    }

    pub async fn inspect_command(&self, request: String) -> Result<String> {
        let request = CompletionRequest {
            model: Some(self.model_ident.clone()),
            messages: Some(vec![
                Message::System {
                    content: Content::Text("The user is going to pass in a command. Your role is to inspect this for safety, evaluating whether or not the command could cause unexpected harm. Unexpected harm may be deleting or removing more files than intended. Your response should first lead with a Y for safe or N for unsafe. Your analysis should be concise, focussing on any caveats first and foremost".into()),
                    name: None,
                },
                Message::User {
                    content: Content::Text(request),
                    name: None,
                }
            ]),
            max_tokens: Some(1000),
            temperature: Some(0.7),
            ..Default::default()
        };

        // TODO: Asset final response is from Assistant

        let response: CompletionResponse = self.completion(request).await?;
        response.assume_single_text_choice()
    }
}
