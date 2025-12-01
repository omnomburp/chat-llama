use axum::{
    Json, Router,
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    routing::post,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tower_http::services::{ServeDir, ServeFile};

// ---------- App state ----------

#[derive(Clone)]
struct AppState {
    llama_base_url: String,
    llama_model: String,
}

impl AppState {
    fn from_env() -> Self {
        Self {
            llama_base_url: std::env::var("LLAMA_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string()),
            llama_model: std::env::var("LLAMA_MODEL").unwrap_or_else(|_| "local-model".to_string()),
        }
    }
}

// ---------- Chat types ----------

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ChatMessage {
    role: String, // "user" | "assistant" | "system"
    content: String,
}

#[derive(Debug, Serialize, Clone)]
struct LlamaMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: ToolCallFunctionCall,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ToolCallFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Default, Debug, Clone)]
struct ToolCallBuilder {
    id: Option<String>,
    function_name: Option<String>,
    arguments: String,
}

impl ToolCallBuilder {
    fn merge_delta(&mut self, delta: &serde_json::Value) {
        if let Some(id) = delta.get("id").and_then(|v| v.as_str()) {
            if self.id.is_none() {
                self.id = Some(id.to_string());
            }
        }
        if let Some(function) = delta.get("function") {
            if let Some(name) = function.get("name").and_then(|v| v.as_str()) {
                if self.function_name.is_none() {
                    self.function_name = Some(name.to_string());
                }
            }
            if let Some(args) = function.get("arguments").and_then(|v| v.as_str()) {
                self.arguments.push_str(args);
            }
        }
    }

    fn build(self) -> Option<ToolCall> {
        let id = self.id?;
        let name = self.function_name?;
        Some(ToolCall {
            id,
            call_type: "function".to_string(),
            function: ToolCallFunctionCall {
                name,
                arguments: self.arguments,
            },
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Clone)]
struct Tool {
    #[serde(rename = "type")]
    tool_type: String,
    function: ToolFunction,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Clone)]
struct ToolFunction {
    name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    description: String,
    parameters: serde_json::Value,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
enum ToolChoice {
    Simple(String),
    Detailed(serde_json::Value),
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    message: String,
    use_search: bool,
    history: Vec<ChatMessage>,
}

#[derive(Debug, Serialize, Clone)]
struct SearchResult {
    title: String,
    snippet: String,
    url: String,
}

// ---------- main ----------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = AppState::from_env();

    // Serve ./dist (built Svelte app).
    // If file not found, serve index.html (SPA fallback).
    let static_files = ServeDir::new("dist").not_found_service(ServeFile::new("dist/index.html"));

    let app = Router::new()
        .route("/api/chat/stream", post(chat_stream_handler))
        .fallback_service(static_files)
        .with_state(Arc::new(state));

    let addr: SocketAddr = "0.0.0.0:3000".parse()?;
    println!("Server running at http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// ---------- Streaming chat endpoint (passes through real llama stream) ----------

async fn chat_stream_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Result<
    Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>,
    (axum::http::StatusCode, String),
> {
    #[derive(Serialize)]
    struct LlamaStreamRequest {
        model: String,
        messages: Vec<LlamaMessage>,
        stream: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        tools: Option<Vec<Tool>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_choice: Option<ToolChoice>,
        #[serde(skip_serializing_if = "Option::is_none")]
        parallel_tool_calls: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        parse_tool_calls: Option<bool>,
    }

    let search_enabled = req.use_search;
    let mut messages = build_llama_messages(&req, search_enabled);
    let tools = if search_enabled {
        Some(vec![web_search_tool_definition()])
    } else {
        None
    };
    let tool_choice = tools
        .as_ref()
        .map(|_| ToolChoice::Simple("auto".to_string()));

    let llama_model = state.llama_model.clone();
    let llama_base_url = state.llama_base_url.clone();
    let client = reqwest::Client::new();

    let event_stream = async_stream::stream! {
        let mut sources: Vec<SearchResult> = Vec::new();
        if let Ok(sources_json) = serde_json::to_string(&sources) {
            yield Ok::<Event, Infallible>(Event::default().event("sources").data(sources_json));
        }

        loop {
            let llama_req = LlamaStreamRequest {
                model: llama_model.clone(),
                messages: messages.clone(),
                stream: true,
                tools: tools.clone(),
                tool_choice: tool_choice.clone(),
                parallel_tool_calls: None,
                parse_tool_calls: tools.as_ref().map(|_| true),
            };

            let url = format!("{}/v1/chat/completions", llama_base_url);
            let resp = match client
                .post(&url)
                .header("Content-Type", "application/json")
                .bearer_auth("no-key")
                .json(&llama_req)
                .send()
                .await
            {
                Ok(resp) => match resp.error_for_status() {
                    Ok(ok) => ok,
                    Err(err) => {
                        eprintln!("llama response error: {err:?}");
                        let ev = Event::default()
                            .event("error")
                            .data("LLM error (see server logs)");
                        yield Ok(ev);
                        return;
                    }
                },
                Err(err) => {
                    eprintln!("llama stream send error: {err:?}");
                    let ev = Event::default()
                        .event("error")
                        .data("LLM streaming error (see server logs)");
                    yield Ok(ev);
                    return;
                }
            };

            let mut byte_stream = resp.bytes_stream();
            let mut buffer = String::new();
            let mut tool_builders: Vec<ToolCallBuilder> = Vec::new();
            let mut saw_tool_calls = false;

            'stream_loop: while let Some(chunk_res) = byte_stream.next().await {
                match chunk_res {
                    Ok(chunk) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));

                        loop {
                            if let Some(idx) = buffer.find("\n\n") {
                                let event_block = buffer[..idx].to_string();
                                buffer = buffer[idx + 2..].to_string();

                                let mut data_payloads = Vec::new();
                                for line in event_block.lines() {
                                    let trimmed = line.trim();
                                    if trimmed.starts_with("data:") {
                                        data_payloads.push(trimmed.trim_start_matches("data:").trim().to_string());
                                    }
                                }

                                for data_str in data_payloads {
                                    if data_str == "[DONE]" {
                                        break 'stream_loop;
                                    }

                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data_str) {
                                        if let Some(choice) = json["choices"].get(0) {
                                            if let Some(delta) = choice.get("delta") {
                                                if let Some(tool_calls) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                                                    saw_tool_calls = true;
                                                    for tc in tool_calls {
                                                        let index = tc.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                                                        if index >= tool_builders.len() {
                                                            tool_builders.resize_with(index + 1, ToolCallBuilder::default);
                                                        }
                                                        tool_builders[index].merge_delta(tc);
                                                    }
                                                    continue;
                                                }

                                                if !saw_tool_calls {
                                                    if let Some(delta_text) = delta
                                                        .get("content")
                                                        .and_then(|c| c.as_str())
                                                    {
                                                        if !delta_text.is_empty() {
                                                            let out_json = serde_json::json!({
                                                                "choices": [{
                                                                    "delta": { "content": delta_text }
                                                                }]
                                                            });
                                                            yield Ok(Event::default().data(out_json.to_string()));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                break;
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("llama chunk error: {err:?}");
                        let ev = Event::default()
                            .event("error")
                            .data("stream error (see server logs)");
                        yield Ok(ev);
                        return;
                    }
                }
            }

            if saw_tool_calls {
                let mut built_calls = Vec::new();
                for builder in tool_builders {
                    if let Some(call) = builder.build() {
                        built_calls.push(call);
                    }
                }

                if built_calls.is_empty() {
                    eprintln!("Tool call indicated but nothing was built");
                    break;
                }

                messages.push(LlamaMessage {
                    role: "assistant".into(),
                    content: None,
                    tool_calls: Some(built_calls.clone()),
                    name: None,
                    tool_call_id: None,
                });

                for call in built_calls {
                    match handle_tool_call(&call).await {
                        Ok((tool_content, maybe_sources)) => {
                            if let Some(new_sources) = maybe_sources {
                                sources = new_sources;
                                if let Ok(json) = serde_json::to_string(&sources) {
                                    yield Ok(Event::default().event("sources").data(json));
                                }
                            }

                            messages.push(LlamaMessage {
                                role: "tool".into(),
                                content: Some(tool_content),
                                tool_calls: None,
                                name: Some(call.function.name.clone()),
                                tool_call_id: Some(call.id.clone()),
                            });
                        }
                        Err(err) => {
                            eprintln!("Tool execution failed: {err:?}");
                            let error_payload = serde_json::json!({
                                "error": format!("tool {name} failed: {err}", name = call.function.name)
                            });
                            messages.push(LlamaMessage {
                                role: "tool".into(),
                                content: Some(error_payload.to_string()),
                                tool_calls: None,
                                name: Some(call.function.name.clone()),
                                tool_call_id: Some(call.id.clone()),
                            });
                        }
                    }
                }

                continue;
            } else {
                break;
            }
        }
    };

    Ok(Sse::new(event_stream).keep_alive(KeepAlive::default()))
}

#[derive(Deserialize)]
struct SearxngSearchResponse {
    results: Vec<SearxngResult>,
}

#[derive(Deserialize)]
struct SearxngResult {
    title: Option<String>,
    url: Option<String>,
    content: Option<String>,
}

use reqwest::Client;

async fn web_search(query: &str) -> anyhow::Result<Vec<SearchResult>> {
    let base_url = std::env::var("SEARCH_BASE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:4434".into());
    let base_url = base_url.trim_end_matches('/').to_owned();

    // Client used for SearXNG API
    let search_client = Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) \
                     AppleWebKit/537.36 (KHTML, like Gecko) \
                     Chrome/123.0.0.0 Safari/537.36")
        .build()?;

    // Client for scraping result pages — no cookies, no referer
    let scrape_client = Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) \
                     AppleWebKit/537.36 (KHTML, like Gecko) \
                     Chrome/123.0.0.0 Safari/537.36")
        // We don't add a cookie store, but we ALSO don't set any cookies
        // (reqwest does not send cookies unless told to).
        .build()?;

    let resp = search_client
        .get(format!("{base_url}/search"))
        .query(&[
            ("q", query),
            ("format", "json"),
            ("language", "en"),
        ])
        .header("Accept", "application/json")
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("search backend error {}: {}", status, body);
    }

    let parsed: SearxngSearchResponse = resp.json().await?;

    let mut results: Vec<SearchResult> = parsed
        .results
        .into_iter()
        .filter_map(|r| {
            let url = r.url?;
            let title = r.title.unwrap_or_else(|| url.clone());
            let snippet = r.content.unwrap_or_default();
            Some(SearchResult { title, snippet, url })
        })
        .take(5)
        .collect();

    for res in results.iter_mut().take(2) {
        if let Some(excerpt) = fetch_page_excerpt(&scrape_client, &res.url).await {
            if res.snippet.is_empty() {
                res.snippet = excerpt;
            } else {
                res.snippet = format!("{} • Page excerpt: {}", res.snippet, excerpt);
            }
        }
    }

    Ok(results)
}

async fn fetch_page_excerpt(client: &Client, url: &str) -> Option<String> {
    use scraper::{Html, Selector};

    // Normal GET — reqwest won't send cookies unless explicitly configured
    let resp = client
        .get(url)
        .header("Accept", "text/html,*/*")
        // IMPORTANT: we intentionally do NOT set Referer
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let body = resp.text().await.ok()?;
    let document = Html::parse_document(&body);
    let body_sel = Selector::parse("body").ok()?;

    let mut text = String::new();
    for node in document.select(&body_sel) {
        text.push_str(&node.text().collect::<String>());
    }

    let cleaned = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() {
        return None;
    }

    Some(cleaned.chars().take(4000).collect())
}


fn web_search_tool_definition() -> Tool {
    Tool {
        tool_type: "function".into(),
        function: ToolFunction {
            name: "web_search".into(),
            description: "Searches the web and returns the top results.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Short search query describing what you need to know"
                    },
                    "max_results": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 5,
                        "description": "Optional maximum number of results to return (default 5)"
                    }
                },
                "required": ["query"]
            }),
        },
    }
}

#[derive(Deserialize)]
struct WebSearchToolArgs {
    query: String,
    #[serde(default)]
    max_results: Option<usize>,
}

async fn handle_tool_call(call: &ToolCall) -> anyhow::Result<(String, Option<Vec<SearchResult>>)> {
    match call.function.name.as_str() {
        "web_search" => {
            let args: WebSearchToolArgs = serde_json::from_str(&call.function.arguments)
                .map_err(|e| anyhow::anyhow!("invalid search args: {e}"))?;
            let trimmed_query = args.query.trim();
            if trimmed_query.is_empty() {
                anyhow::bail!("search query missing");
            }
            let mut results = web_search(trimmed_query).await?;
            let limit = args.max_results.unwrap_or(5).clamp(1, 7);
            if results.len() > limit {
                results.truncate(limit);
            }
            let payload = format_search_results_for_tool(&results, trimmed_query);
            Ok((payload, Some(results)))
        }
        other => {
            anyhow::bail!("unknown tool call: {other}");
        }
    }
}

fn format_search_results_for_tool(results: &[SearchResult], query: &str) -> String {
    let entries: Vec<_> = results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            serde_json::json!({
                "id": i + 1,
                "title": r.title,
                "snippet": r.snippet,
                "url": r.url,
            })
        })
        .collect();

    serde_json::json!({
        "query": query,
        "results": entries,
        "instructions": "Use the snippets and cite sources like [id] in your response."
    })
    .to_string()
}

// ---------- Non-streaming call to llama-server ----------

fn build_llama_messages(req: &ChatRequest, search_enabled: bool) -> Vec<LlamaMessage> {
    let mut messages = Vec::<LlamaMessage>::new();

    let system_prompt = if search_enabled {
        "You are a helpful AI assistant. You can call the web_search tool to fetch recent web information.\n\
Use the tool whenever the user asks for factual data you are unsure about.\n\
When citing information derived from tool results, refer to them as [n] where n is the result index."
    } else {
        "You are a helpful AI assistant. Answer as clearly as possible using only your existing knowledge."
    };

    messages.push(LlamaMessage {
        role: "system".into(),
        content: Some(system_prompt.into()),
        tool_calls: None,
        name: None,
        tool_call_id: None,
    });

    for m in &req.history {
        messages.push(LlamaMessage {
            role: m.role.clone(),
            content: Some(m.content.clone()),
            tool_calls: None,
            name: None,
            tool_call_id: None,
        });
    }

    messages.push(LlamaMessage {
        role: "user".into(),
        content: Some(req.message.clone()),
        tool_calls: None,
        name: None,
        tool_call_id: None,
    });

    messages
}
