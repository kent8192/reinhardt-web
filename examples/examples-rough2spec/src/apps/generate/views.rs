//! generate app view handlers
//!
//! POST /api/generate  — Convert rough idea to structured spec JSON
//! GET  /api/health    — Health check

use reinhardt::core::serde::json;
use reinhardt::http::ViewResult;
use reinhardt::{Json, Response, StatusCode};
use reinhardt::{get, post};
use serde::{Deserialize, Serialize};

// ─── Request / Response types ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct GenerateRequest {
    pub idea: String,
    pub template: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Spec {
    pub product_name: String,
    pub target_user: String,
    pub problem: String,
    pub value_prop: String,
    pub core_features: Vec<String>,
    pub non_goals: Vec<String>,
    pub user_stories: Vec<String>,
    pub tasks: Vec<String>,
    pub builder_prompt: String,
}

#[derive(Debug, Serialize)]
struct GenerateResponse {
    spec: Spec,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

// ─── Endpoints ─────────────────────────────────────────────────────────

/// Health check endpoint
#[get("/health", name = "health")]
pub async fn health() -> ViewResult<Response> {
    let body = json::json!({"status": "ok"});
    Ok(Response::new(StatusCode::OK)
        .with_header("Content-Type", "application/json")
        .with_body(json::to_vec(&body)?))
}

/// Convert a rough idea into a structured spec
///
/// POST /api/generate
/// Body: { "idea": "...", "template": "SaaS|業務ツール|EC|メディア|モバイルアプリ" }
#[post("/generate", name = "generate")]
pub async fn generate(Json(req): Json<GenerateRequest>) -> ViewResult<Response> {
    let idea = req.idea.trim().to_string();
    if idea.is_empty() {
        let body = ErrorResponse {
            error: "idea is required".to_string(),
        };
        return Ok(Response::new(StatusCode::BAD_REQUEST)
            .with_header("Content-Type", "application/json")
            .with_header("Access-Control-Allow-Origin", "*")
            .with_body(json::to_vec(&body)?));
    }

    match call_lm_studio(&idea, &req.template).await {
        Ok(spec) => {
            let resp = GenerateResponse { spec };
            Ok(Response::new(StatusCode::OK)
                .with_header("Content-Type", "application/json")
                .with_header("Access-Control-Allow-Origin", "*")
                .with_body(json::to_vec(&resp)?))
        }
        Err(e) => {
            let body = ErrorResponse {
                error: e.to_string(),
            };
            Ok(Response::new(StatusCode::INTERNAL_SERVER_ERROR)
                .with_header("Content-Type", "application/json")
                .with_header("Access-Control-Allow-Origin", "*")
                .with_body(json::to_vec(&body)?))
        }
    }
}

// ─── LM Studio API call (OpenAI-compatible) ────────────────────────────

const LM_STUDIO_URL: &str = "http://localhost:1234/v1/chat/completions";
const LM_STUDIO_MODEL: &str = "glm-edge-v-5b";

async fn call_lm_studio(idea: &str, template: &str) -> anyhow::Result<Spec> {
    let prompt = build_prompt(idea, template);

    #[derive(Serialize)]
    struct Message {
        role: String,
        content: String,
    }

    #[derive(Serialize)]
    struct ChatRequest {
        model: String,
        max_tokens: u32,
        messages: Vec<Message>,
    }

    #[derive(Deserialize)]
    struct ChatChoice {
        message: ChatMessage,
    }

    #[derive(Deserialize)]
    struct ChatMessage {
        content: String,
    }

    #[derive(Deserialize)]
    struct ChatResponse {
        choices: Vec<ChatChoice>,
    }

    let client = reqwest::Client::new();
    let request = ChatRequest {
        model: LM_STUDIO_MODEL.to_string(),
        max_tokens: 1800,
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt,
        }],
    };

    let resp = client
        .post(LM_STUDIO_URL)
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await?;
        anyhow::bail!("LM Studio API error {}: {}", status, body);
    }

    let chat_resp: ChatResponse = resp.json().await?;
    let text = chat_resp
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Empty response from LM Studio"))?
        .message
        .content;
    let text = text.trim().to_string();

    // Strip <think> tags (some reasoning models emit these)
    let text = if let Some(e) = text.find("</think>") {
        text[e + 8..].trim().to_string()
    } else {
        text
    };

    // Extract JSON from response
    let json_text = if let Some(start) = text.find('{') {
        let end = text.rfind('}').unwrap_or(text.len());
        text[start..=end].to_string()
    } else {
        text
    };

    let spec: Spec = serde_json::from_str(&json_text).map_err(|e| {
        anyhow::anyhow!("Failed to parse spec JSON: {}. Response: {}", e, json_text)
    })?;

    Ok(spec)
}

fn build_prompt(idea: &str, template: &str) -> String {
    format!(
        r##"/no_think
You are a product manager and tech lead.
Create a high-precision specification from the following rough idea that can be handed to an engineer.

## Idea
{idea}

## Template Category
{template}

## Output Format (JSON only, no explanations)
Strictly follow the JSON schema below. Output ONLY JSON, no other characters.

{{
  "product_name": "Short memorable product name",
  "target_user": "Specific target user persona (role, scale, pain point)",
  "problem": "Problem to solve (specific pain, cost, inefficiency)",
  "value_prop": "Value provided (1-2 sentences including differentiation)",
  "core_features": [
    "Feature 1 (verb-first, specific)",
    "Feature 2",
    "Feature 3"
  ],
  "non_goals": [
    "Out-of-scope feature 1 (with reason)",
    "Out-of-scope feature 2"
  ],
  "user_stories": [
    "As a [role], I want to [goal], so that [benefit].",
    "As a [role], I want to [goal], so that [benefit].",
    "As a [role], I want to [goal], so that [benefit]."
  ],
  "tasks": [
    "[BE] Implement POST /api/xxx endpoint (req: {{field}}, res: {{field}})",
    "[FE] Implement xxx screen (components: xxx, state: xxx)",
    "[DB] Design xxx table migration (columns: id, xxx, created_at)",
    "[AUTH] Implement xxx auth flow (library: xxx)",
    "[TEST] Write unit tests for xxx (coverage target: 80%)",
    "[DEPLOY] Deploy to xxx environment (infra: xxx)",
    "[UX] Create wireframe for xxx"
  ],
  "builder_prompt": "# Implementation Guide\n\nImplement the following features:\n\n1. [Specific feature with technical details]\n2. [Specific feature with technical details]"
}}

Important:
- tasks must include specific endpoint names, column names, library names (not vague descriptions)
- builder_prompt must be a complete instruction that can be copy-pasted directly to Claude Code/Codex
- Output JSON only (no markdown code blocks)"##,
        idea = idea,
        template = template,
    )
}
