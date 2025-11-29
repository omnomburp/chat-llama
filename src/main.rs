use axum::{
    Json, Router,
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    routing::post,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, fmt::Write as _, net::SocketAddr, sync::Arc};
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

// ---------- Streaming chat endpoint (fake streaming over full reply) ----------

async fn chat_stream_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Result<
    Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>,
    (axum::http::StatusCode, String),
> {
    let sources = maybe_execute_search(&state, &req, "chat_stream_handler").await;
    let messages = build_llama_messages(&req, &sources);

    #[derive(Serialize)]
    struct LlamaStreamRequest {
        model: String,
        messages: Vec<ChatMessage>,
        stream: bool,
    }

    let llama_req = LlamaStreamRequest {
        model: state.llama_model.clone(),
        messages,
        stream: true, // IMPORTANT: stream from llama-server
    };

    // ---- 3. Send request to llama-server with stream:true ----
    let url = format!("{}/v1/chat/completions", state.llama_base_url);
    let client = reqwest::Client::new();

    let resp = client
        .post(url)
        .header("Content-Type", "application/json")
        .bearer_auth("no-key") // llama-server ignores this
        .json(&llama_req)
        .send()
        .await
        .map_err(|e| {
            eprintln!("llama stream send error: {e:?}");
            (
                axum::http::StatusCode::BAD_GATEWAY,
                "LLM streaming error".to_string(),
            )
        })?;

    let mut byte_stream = resp.bytes_stream();

    // ---- 4. Build SSE stream we send to the browser ----
    let sources_json = serde_json::to_string(&sources).unwrap_or_else(|_| "[]".to_string());

    let event_stream = async_stream::stream! {
        // First, send the search sources as a custom "sources" event
        yield Ok::<Event, Infallible>(Event::default().event("sources").data(sources_json));

        let mut buffer = String::new();

        while let Some(chunk_res) = byte_stream.next().await {
            match chunk_res {
                Ok(chunk) => {
                    buffer.push_str(&String::from_utf8_lossy(&chunk));

                    // Process complete SSE events from llama (split by blank line)
                    loop {
                        if let Some(idx) = buffer.find("\n\n") {
                            let event_block = buffer[..idx].to_string();
                            buffer = buffer[idx + 2..].to_string(); // skip "\n\n"

                            // Each block may contain "data: ..." lines
                            for line in event_block.lines() {
                                let line = line.trim();
                                if !line.starts_with("data:") {
                                    continue;
                                }

                                let data_str = line.trim_start_matches("data:").trim();
                                if data_str == "[DONE]" {
                                    // Llama finished streaming
                                    return;
                                }

                                // Parse llama's JSON chunk, extract delta.content
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data_str) {
                                    if let Some(delta) = json["choices"]
                                        .get(0)
                                        .and_then(|c| c.get("delta"))
                                        .and_then(|d| d.get("content"))
                                        .and_then(|c| c.as_str())
                                    {
                                        // Re-wrap into a simple JSON chunk the frontend expects
                                        let out_json = serde_json::json!({
                                            "choices": [{
                                                "delta": { "content": delta }
                                            }]
                                        });
                                        yield Ok(Event::default().data(out_json.to_string()));
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
                    // Optionally send an error event and then end
                    let ev = Event::default()
                        .event("error")
                        .data("stream error (see server logs)");
                    yield Ok(ev);
                    return;
                }
            }
        }
    };

    Ok(Sse::new(event_stream).keep_alive(KeepAlive::default()))
}

async fn maybe_execute_search(
    state: &AppState,
    req: &ChatRequest,
    caller: &str,
) -> Vec<SearchResult> {
    if !req.use_search {
        println!("{caller}: use_search = false");
        return Vec::new();
    }

    match plan_search(state, req).await {
        Ok(Some(search_query)) => {
            println!("{caller}: planner chose search query = {:?}", search_query);
            match duckduckgo_search(&search_query).await {
                Ok(results) => {
                    println!("{caller}: got {} sources", results.len());
                    results
                }
                Err(e) => {
                    eprintln!("{caller}: search failed: {e:?}");
                    Vec::new()
                }
            }
        }
        Ok(None) => {
            println!("{caller}: planner chose no search");
            Vec::new()
        }
        Err(e) => {
            eprintln!("{caller}: plan_search error: {e:?}");
            Vec::new()
        }
    }
}

async fn duckduckgo_search(query: &str) -> anyhow::Result<Vec<SearchResult>> {
    use scraper::{Html, Selector};

    // DuckDuckGo's HTML-only search page (no JS)
    let url = format!(
        "https://duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );

    let resp = reqwest::get(&url).await?.error_for_status()?;
    let body = resp.text().await?;
    let document = Html::parse_document(&body);

    // Selectors based on DuckDuckGo's HTML structure
    let result_sel = Selector::parse("div.result").unwrap();
    let title_sel = Selector::parse("a.result__a").unwrap();
    let snippet_sel = Selector::parse("a.result__snippet, div.result__snippet").unwrap();

    let mut results = Vec::new();

    for result in document.select(&result_sel).take(6) {
        let title_el = result.select(&title_sel).next();
        let snippet_el = result.select(&snippet_sel).next();

        if let Some(title_el) = title_el {
            let title = title_el.text().collect::<String>().trim().to_string();
            let url = title_el.value().attr("href").unwrap_or("").to_string();
            let snippet = snippet_el
                .map(|s| s.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if !url.is_empty() {
                results.push(SearchResult {
                    title,
                    snippet,
                    url,
                });
            }
        }
    }

    Ok(results)
}

// ---------- Non-streaming call to llama-server ----------

fn build_llama_messages(req: &ChatRequest, sources: &[SearchResult]) -> Vec<ChatMessage> {
    let mut messages = Vec::<ChatMessage>::new();

    let system_prompt = if sources.is_empty() {
        "You are a helpful AI assistant. Answer as clearly as possible."
    } else {
        "You are a helpful AI assistant with access to web search results. \
         Use the provided snippets as context. If something is unclear, say so."
    };

    messages.push(ChatMessage {
        role: "system".into(),
        content: system_prompt.into(),
    });

    for m in &req.history {
        messages.push(m.clone());
    }

    let mut user_content = String::new();

    if !sources.is_empty() {
        user_content.push_str("Web search results:\n");
        for (i, s) in sources.iter().enumerate() {
            let _ = writeln!(
                &mut user_content,
                "[{}] {} — {}\n{}",
                i + 1,
                s.title,
                s.snippet,
                s.url
            );
        }
        user_content.push_str("\n\nBased on these results, answer the user question:\n");
    }

    user_content.push_str(&req.message);

    messages.push(ChatMessage {
        role: "user".into(),
        content: user_content,
    });

    messages
}

#[derive(Serialize)]
struct LlamaChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Deserialize)]
struct LlamaChatResponse {
    choices: Vec<LlamaChoice>,
}

#[derive(Deserialize)]
struct LlamaChoice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct SearchPlannerResponse {
    use_search: bool,
    #[serde(default)]
    query: String,
}

async fn plan_search(state: &AppState, req: &ChatRequest) -> anyhow::Result<Option<String>> {
    // Ask the model whether to use search and what query to use.
    // It must answer with pure JSON: {"use_search": bool, "query": "..."}

    let mut messages = Vec::<ChatMessage>::new();

    messages.push(ChatMessage {
        role: "system".into(),
        content: r#"You are a search planner for a web-enabled assistant.

Given the conversation so far and the user's latest message, decide whether web search is needed.

You MUST answer ONLY with a single JSON object, no extra text, in this exact format:

{"use_search": false}

OR

{"use_search": true, "query": "<short search query>"}

Rules:
- use_search should be true only if recent or factual web info is needed.
- query should be a concise search query (not the whole conversation).
- Never include explanations or other text outside the JSON."#
            .into(),
    });

    // Include prior messages (without previous search results).
    for m in &req.history {
        messages.push(m.clone());
    }

    // Latest user message
    messages.push(ChatMessage {
        role: "user".into(),
        content: req.message.clone(),
    });

    let llama_req = LlamaChatRequest {
        model: state.llama_model.clone(),
        messages,
        stream: false,
    };

    let url = format!("{}/v1/chat/completions", state.llama_base_url);
    let client = reqwest::Client::new();

    let resp = client
        .post(url)
        .header("Content-Type", "application/json")
        .bearer_auth("no-key")
        .json(&llama_req)
        .send()
        .await?
        .error_for_status()?;

    let llama_resp: LlamaChatResponse = resp.json().await?;
    let content = llama_resp
        .choices
        .get(0)
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    match parse_planner_json(&content) {
        Ok(result) => return Ok(result),
        Err(_) => {
            eprintln!("plan_search: failed to parse planner JSON: {content}");
        }
    }

    if let Some(result) = retry_planner_parse(state, &content).await? {
        return Ok(Some(result));
    }

    Ok(None) // fall back to no search if planner response is bad
}

fn parse_planner_json(raw: &str) -> Result<Option<String>, serde_json::Error> {
    let parsed = serde_json::from_str::<SearchPlannerResponse>(raw)?;
    if !parsed.use_search {
        Ok(None)
    } else {
        let q = parsed.query.trim();
        if q.is_empty() {
            Ok(None)
        } else {
            Ok(Some(q.to_string()))
        }
    }
}

async fn retry_planner_parse(
    state: &AppState,
    planner_output: &str,
) -> anyhow::Result<Option<String>> {
    let mut messages = Vec::<ChatMessage>::new();
    messages.push(ChatMessage {
        role: "system".into(),
        content: r#"You fix invalid JSON from a search planner.
Return ONLY valid JSON matching: {"use_search": bool, "query": "optional string"}.
If the planner text implies search is required, set use_search true and craft a short query.
If no search is needed, return {"use_search": false}."#
            .into(),
    });

    messages.push(ChatMessage {
        role: "user".into(),
        content: format!(
            "Planner output:\n\n{}\n\nReturn corrected JSON only.",
            planner_output
        ),
    });

    let llama_req = LlamaChatRequest {
        model: state.llama_model.clone(),
        messages,
        stream: false,
    };

    let url = format!("{}/v1/chat/completions", state.llama_base_url);
    let client = reqwest::Client::new();

    let resp = client
        .post(url)
        .header("Content-Type", "application/json")
        .bearer_auth("no-key")
        .json(&llama_req)
        .send()
        .await?
        .error_for_status()?;

    let llama_resp: LlamaChatResponse = resp.json().await?;
    let fallback_content = llama_resp
        .choices
        .get(0)
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    match parse_planner_json(&fallback_content) {
        Ok(result) => Ok(result),
        Err(_) => {
            eprintln!(
                "retry_planner_parse: failed to parse cleaned planner response: {fallback_content}"
            );
            Ok(None)
        }
    }
}
