/// 该 provider 的 catalog 条目。
pub fn catalog_entry() -> crate::catalog::CatalogEntry {
    crate::catalog::CatalogEntry {
        api_key: Vec::new(),
        name: "Google Gemini".into(),
        kind: crate::catalog::ProviderKind::Gemini,
        base_url: Some("https://generativelanguage.googleapis.com".into()),
        api_key_env: Some("GEMINI_API_KEY".into()),
        default_model: Some("gemini-2.0-flash".into()),
        extra_body: None,
    }
}

use async_trait::async_trait;
use ring_core::tools::{ContentBlock, Message, MessageRole};
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;
use tracing::debug;

use crate::error::ProviderError;
use crate::provider::{
    check_response_error, ChatRequest, ChatResponse, ModelInfo, Provider, ProviderStream,
    StopReason, StreamChunk, StreamEvent, ToolDef, Usage,
};

const GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com";
const CONNECT_TIMEOUT_SECS: u64 = 10;
const DEFAULT_MODEL: &str = "gemini-2.0-flash";
const DEFAULT_THINKING_BUDGET: u32 = 16_384;

fn effort_to_thinking_budget(effort: Option<&str>) -> u32 {
    match effort {
        Some("max")                => 24_576,
        Some("xhigh")              => 24_576,
        Some("high")               => 16_384,
        Some("medium")             => 8_192,
        Some("low") | Some("minimal") => 4_096,
        _                          => DEFAULT_THINKING_BUDGET,
    }
}

/// 把内部消息列表转为 Gemini `contents` 数组。
///
/// 先扫描所有 ToolUse 建立 tool_use_id → tool_name 映射，
/// 以便 ToolResult 能正确填写 functionResponse.name。
fn convert_messages_to_contents(msgs: &[Message]) -> Value {
    let mut tool_names: HashMap<String, String> = HashMap::new();
    for msg in msgs {
        for blk in &msg.content {
            if let ContentBlock::ToolUse { tool_use_id, tool_name, .. } = blk {
                tool_names.insert(tool_use_id.clone(), tool_name.clone());
            }
        }
    }

    let mut contents = Vec::new();
    for msg in msgs {
        match msg.role {
            MessageRole::User => {
                let parts: Vec<Value> = msg.content.iter()
                    .filter_map(|blk| match blk {
                        ContentBlock::Text { text } => Some(json!({ "text": text })),
                        ContentBlock::Image { media_type, data } => Some(json!({
                            "inline_data": { "mime_type": media_type, "data": data }
                        })),
                        _ => None,
                    })
                    .collect();
                if !parts.is_empty() {
                    contents.push(json!({ "role": "user", "parts": parts }));
                }
            }
            MessageRole::Assistant => {
                let parts: Vec<Value> = msg.content.iter()
                    .filter_map(|blk| match blk {
                        ContentBlock::Text { text } => Some(json!({ "text": text })),
                        ContentBlock::ToolUse { tool_name, tool_input, .. } => Some(json!({
                            "functionCall": { "name": tool_name, "args": tool_input }
                        })),
                        ContentBlock::Thinking { .. }
                        | ContentBlock::ToolResult { .. }
                        | ContentBlock::Image { .. } => None,
                    })
                    .collect();
                if !parts.is_empty() {
                    contents.push(json!({ "role": "model", "parts": parts }));
                }
            }
            MessageRole::ToolResult => {
                let parts: Vec<Value> = msg.content.iter()
                    .filter_map(|blk| match blk {
                        ContentBlock::ToolResult { tool_use_id, tool_result, is_error } => {
                            let name = tool_names.get(tool_use_id).cloned().unwrap_or_default();
                            let mut response = json!({ "result": tool_result });
                            if *is_error {
                                response["isError"] = json!(true);
                            }
                            Some(json!({
                                "functionResponse": { "name": name, "response": response }
                            }))
                        }
                        _ => None,
                    })
                    .collect();
                if !parts.is_empty() {
                    contents.push(json!({ "role": "user", "parts": parts }));
                }
            }
        }
    }
    Value::Array(contents)
}

fn convert_tools(tools: &[ToolDef]) -> Value {
    let function_declarations: Vec<Value> = tools
        .iter()
        .map(|t| json!({
            "name": t.name,
            "description": t.description,
            "parameters": t.input_schema,
        }))
        .collect();
    json!([{ "function_declarations": function_declarations }])
}

fn parse_finish_reason(s: Option<&str>, has_function_call: bool) -> StopReason {
    if has_function_call {
        return StopReason::ToolUse;
    }
    match s {
        Some("STOP")       => StopReason::EndTurn,
        Some("MAX_TOKENS") => StopReason::MaxTokens,
        _                  => StopReason::EndTurn,
    }
}

pub struct GeminiProvider {
    client:    Client,
    api_key:   String,
    base_url:  String,
    def_model: String,
}

impl GeminiProvider {
    pub fn new(api_key: impl Into<String>, base_url: Option<String>, def_model: Option<String>) -> Self {
        let client = crate::provider::build_http_client(None, CONNECT_TIMEOUT_SECS);
        Self::with_client(client, api_key, base_url, def_model)
    }

    pub fn with_client(
        client:    Client,
        api_key:   impl Into<String>,
        base_url:  Option<String>,
        def_model: Option<String>,
    ) -> Self {
        Self {
            client,
            api_key: api_key.into(),
            base_url: base_url.unwrap_or_else(|| GEMINI_BASE_URL.to_string()),
            def_model: def_model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        }
    }

    fn endpoint(&self, model: &str, stream: bool) -> String {
        let action = if stream { "streamGenerateContent" } else { "generateContent" };
        format!(
            "{}/v1beta/models/{}:{}?key={}",
            self.base_url, model, action, self.api_key
        )
    }

    fn build_body(&self, req: &ChatRequest) -> Value {
        let mut gen_config = json!({
            "maxOutputTokens": req.max_tokens,
        });
        if let Some(t) = req.temperature {
            gen_config["temperature"] = json!(t);
        }
        if let Some(p) = req.top_p {
            gen_config["topP"] = json!(p);
        }
        if !req.stop.is_empty() {
            gen_config["stopSequences"] = json!(req.stop);
        }
        if req.extended_thinking || req.reasoning_effort.is_some() {
            let budget = req.thinking_budget
                .unwrap_or_else(|| effort_to_thinking_budget(req.reasoning_effort.as_deref()));
            gen_config["thinkingConfig"] = json!({
                "thinkingBudget": budget,
                "includeThoughts": true
            });
        }

        let mut body = json!({
            "contents": convert_messages_to_contents(&req.messages),
            "generationConfig": gen_config,
        });
        if let Some(sys) = &req.system {
            body["systemInstruction"] = json!({ "parts": [{ "text": sys }] });
        }
        if !req.tools.is_empty() {
            body["tools"] = convert_tools(&req.tools);
        }
        body
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    fn id(&self) -> &str { "gemini" }
    fn display_name(&self) -> &str { "Google Gemini" }
    fn default_model(&self) -> &str { &self.def_model }

    async fn chat(&self, req: &ChatRequest, signal: CancellationToken) -> Result<ChatResponse, ProviderError> {
        let url  = self.endpoint(&req.model, false);
        let body = self.build_body(req);
        debug!(model = %req.model, "gemini chat request");

        let resp = tokio::select! {
            r = self.client.post(&url).json(&body).send() => r.map_err(ProviderError::Network)?,
            _ = signal.cancelled() => return Err(ProviderError::Cancelled),
        };
        let resp = check_response_error(resp).await?;
        let raw: Value = resp.json().await.map_err(ProviderError::Network)?;

        let candidate = raw["candidates"]
            .as_array()
            .and_then(|a| a.first())
            .ok_or_else(|| ProviderError::Other("no candidates in response".into()))?;

        let finish_reason = candidate["finishReason"].as_str();

        let usage = Usage {
            input_tokens:          raw["usageMetadata"]["promptTokenCount"].as_u64().unwrap_or(0),
            output_tokens:         raw["usageMetadata"]["candidatesTokenCount"].as_u64().unwrap_or(0),
            cache_creation_tokens: 0,
            cache_read_tokens:     0,
        };

        let mut content_blocks = Vec::new();
        let mut has_fc = false;
        if let Some(parts) = candidate["content"]["parts"].as_array() {
            for part in parts {
                if let Some(text) = part["text"].as_str() {
                    if part["thought"].as_bool() == Some(true) {
                        content_blocks.push(ContentBlock::Thinking {
                            thinking: text.to_string(),
                            signature: None,
                            redacted: false,
                        });
                    } else {
                        content_blocks.push(ContentBlock::Text { text: text.to_string() });
                    }
                }
                if let Some(fc) = part.get("functionCall") {
                    has_fc = true;
                    let name  = fc["name"].as_str().unwrap_or("").to_string();
                    let input = fc["args"].clone();
                    content_blocks.push(ContentBlock::ToolUse {
                        tool_use_id: uuid::Uuid::new_v4().to_string(),
                        tool_name:   name,
                        tool_input:  input,
                    });
                }
            }
        }

        let stop_reason = parse_finish_reason(finish_reason, has_fc);
        let message = Message::new(MessageRole::Assistant, content_blocks);
        Ok(ChatResponse { message, stop_reason, usage, model: req.model.clone() })
    }

    async fn stream(&self, req: &ChatRequest, signal: CancellationToken) -> Result<ProviderStream, ProviderError> {
        let url  = self.endpoint(&req.model, true);
        let body = self.build_body(req);

        let resp = tokio::select! {
            r = self.client.post(&url).header("Accept", "text/event-stream").json(&body).send() => r.map_err(ProviderError::Network)?,
            _ = signal.cancelled() => return Err(ProviderError::Cancelled),
        };
        let resp = check_response_error(resp).await?;
        let (tx, rx) = tokio::sync::mpsc::channel::<StreamEvent>(64);
        let byte_stream = resp.bytes_stream();
        tokio::spawn(run_gemini_sse(byte_stream, signal, tx));
        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(gemini_known_models())
    }
}

async fn run_gemini_sse<S>(
    byte_stream: S,
    signal:      CancellationToken,
    tx:          tokio::sync::mpsc::Sender<StreamEvent>,
) where
    S: futures_util::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
{
    tokio::pin!(byte_stream);
    let mut buf = String::new();
    let mut input_tokens  = 0u64;
    let mut output_tokens = 0u64;
    let mut final_stop = StopReason::EndTurn;
    let mut has_fc = false;

    loop {
        tokio::select! {
            _ = signal.cancelled() => {
                let _ = tx.send(StreamEvent::Error("cancelled".into())).await;
                return;
            }
            chunk = futures_util::StreamExt::next(&mut byte_stream) => {
                let bytes = match chunk {
                    None => break,
                    Some(Err(e)) => { let _ = tx.send(StreamEvent::Error(e.to_string())).await; return; }
                    Some(Ok(b)) => b,
                };
                buf.push_str(&String::from_utf8_lossy(&bytes));

                while let Some(end) = buf.find("\n\n") {
                    let block = buf[..end].to_string();
                    buf = buf[end + 2..].to_string();

                    let data = block.lines()
                        .find_map(|l| l.strip_prefix("data: ").map(str::to_string));
                    let data = match data { Some(d) => d, None => continue };

                    let raw: Value = match serde_json::from_str(&data) { Ok(v) => v, Err(_) => continue };

                    if let Some(u) = raw.get("usageMetadata") {
                        input_tokens  = u["promptTokenCount"].as_u64().unwrap_or(input_tokens);
                        output_tokens = u["candidatesTokenCount"].as_u64().unwrap_or(output_tokens);
                    }

                    let candidate = match raw["candidates"].as_array().and_then(|a| a.first()) {
                        Some(c) => c.clone(),
                        None => continue,
                    };

                    if let Some(fr) = candidate["finishReason"].as_str() {
                        if !fr.is_empty() {
                            final_stop = match fr {
                                "MAX_TOKENS" => StopReason::MaxTokens,
                                _            => StopReason::EndTurn,
                            };
                        }
                    }

                    if let Some(parts) = candidate["content"]["parts"].as_array() {
                        for part in parts {
                            if let Some(text) = part["text"].as_str() {
                                if !text.is_empty() {
                                    if part["thought"].as_bool() == Some(true) {
                                        let _ = tx.send(StreamEvent::Chunk(
                                            StreamChunk::ThinkingDelta { delta: text.to_string() },
                                        )).await;
                                    } else {
                                        let _ = tx.send(StreamEvent::Chunk(
                                            StreamChunk::TextDelta { delta: text.to_string() },
                                        )).await;
                                    }
                                }
                            }
                            if let Some(fc) = part.get("functionCall") {
                                has_fc = true;
                                let name = fc["name"].as_str().unwrap_or("").to_string();
                                let args = fc["args"].clone();
                                let call_id = uuid::Uuid::new_v4().to_string();
                                let _ = tx.send(StreamEvent::Chunk(StreamChunk::ToolCallStart {
                                    call_id:   call_id.clone(),
                                    tool_name: name,
                                })).await;
                                let args_str = args.to_string();
                                let _ = tx.send(StreamEvent::Chunk(StreamChunk::ToolCallInput {
                                    call_id: call_id.clone(),
                                    delta:   args_str,
                                })).await;
                                let _ = tx.send(StreamEvent::Chunk(StreamChunk::ToolCallDone {
                                    call_id,
                                    full_input: args,
                                })).await;
                            }
                        }
                    }
                }
            }
        }
    }
    if has_fc {
        final_stop = StopReason::ToolUse;
    }
    let _ = tx.send(StreamEvent::Done {
        stop_reason: final_stop,
        usage: Usage { input_tokens, output_tokens, ..Usage::default() },
    }).await;
}

fn gemini_known_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "gemini-2.0-flash".into(),
            display_name: "Gemini 2.0 Flash".into(),
            context_window: 1_000_000,
            max_output_tokens: 8_192,
            supports_vision: true,
            supports_thinking: false,
            supports_tools: true,
        },
        ModelInfo {
            id: "gemini-2.0-flash-lite".into(),
            display_name: "Gemini 2.0 Flash-Lite".into(),
            context_window: 1_000_000,
            max_output_tokens: 8_192,
            supports_vision: true,
            supports_thinking: false,
            supports_tools: true,
        },
        ModelInfo {
            id: "gemini-2.5-flash".into(),
            display_name: "Gemini 2.5 Flash".into(),
            context_window: 1_000_000,
            max_output_tokens: 65_536,
            supports_vision: true,
            supports_thinking: true,
            supports_tools: true,
        },
        ModelInfo {
            id: "gemini-2.5-pro".into(),
            display_name: "Gemini 2.5 Pro".into(),
            context_window: 1_000_000,
            max_output_tokens: 65_536,
            supports_vision: true,
            supports_thinking: true,
            supports_tools: true,
        },
    ]
}
