# examples-rough2spec

A Rough2Spec example demonstrating Reinhardt framework with local LLM integration.

**Converts rough Japanese business ideas → structured PRD / Tasks / Builder Prompts**

> "ChatGPT returns text. Rough2Spec returns structure you can hand to an engineer."

## Features

- Django-style project structure: `config/`, `apps/`, `src/bin/manage.rs`
- REST API with JSON I/O
- OpenAI-compatible LLM backend (LM Studio)
- No database, no auth (MVP scope)

## Project Structure

```
src/
├── config/
│   ├── apps.rs       # Installed apps
│   ├── settings.rs   # Settings loader
│   ├── urls.rs       # URL routing
│   └── views.rs      # Root-level views
├── apps/
│   └── generate/
│       ├── urls.rs   # App-level URL patterns
│       └── views.rs  # generate endpoint + LLM call
├── bin/
│   └── manage.rs     # Entry point (manage.py equivalent)
└── lib.rs
```

## Setup

### Prerequisites

- Rust 2024 edition
- [LM Studio](https://lmstudio.ai) running locally with a model loaded

### Quick Start

```bash
# 1. Start LM Studio with a model (e.g., glm-edge-v-5b)
# 2. Run the server
cargo run --bin examples-rough2spec -- runserver
```

## API Endpoints

### GET /api/health

```json
{"status": "ok"}
```

### POST /api/generate

**Request:**
```json
{
  "idea": "飲食店向けのリアルタイム空席予約アプリ",
  "template": "モバイルアプリ"
}
```

**Response:**
```json
{
  "spec": {
    "product_name": "TableReservationApp",
    "target_user": "...",
    "problem": "...",
    "value_prop": "...",
    "core_features": ["..."],
    "non_goals": ["..."],
    "user_stories": ["..."],
    "tasks": ["..."],
    "builder_prompt": "..."
  }
}
```

## Environment

Configure `settings/local.toml` (copy from `settings/local.example.toml`):

```toml
[lm_studio]
url = "http://localhost:1234/v1/chat/completions"
model = "glm-edge-v-5b"
max_tokens = 1800
```

## Running Tests

```bash
cargo nextest run --package examples-rough2spec
```
