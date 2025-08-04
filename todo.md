Aggregator Project TODOs for Axum Setup and Domain Logic
Project Setup

Initialize Rust Project (Priority: High, Effort: ~1 hour)

Create a new Rust project with cargo init aggregator.
Add dependencies to Cargo.toml: axum, tokio, serde, reqwest, tower-http (for middleware like rate limiting), async-trait, dashmap (for in-memory storage).
Set up workspace structure with subcrates for api, service, adapters, and storage.
Configure tokio runtime with #[tokio::main] for async support.


Set Up Basic Axum Server (Priority: High, Effort: ~2 hours)

Create a basic Axum server with a /health endpoint to verify setup.
Configure server to listen on configurable host/port (e.g., 0.0.0.0:3000) via environment variables.
Add basic logging with tracing or log for request debugging.
Example:use axum::{routing::get, Router};
async fn health() -> &'static str { "OK" }
#[tokio::main]
async fn main() {
    let app = Router::new().route("/health", get(health));
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}




Configure Environment-Based Settings (Priority: High, Effort: ~2 hours)

Implement configuration loading from .env or JSON file using dotenvy or config.
Define settings for server host/port, solver endpoints, timeouts (per-solver: 1000–3000 ms, global: 3000–5000 ms), and environment profiles (dev/staging/prod).
Example fields: server.host, server.port, solvers[].endpoint, solvers[].timeout_ms.


Set Up Project Structure (Priority: High, Effort: ~2 hours)

Organize codebase into modules: api (routes), service (aggregation logic), adapters (solver plugins), storage (in-memory store), auth (IP-based rate limiting).
Create a lib.rs to expose public APIs and a main.rs for the server entry point.
Example structure:src/
  ├── api/
  │   └── mod.rs
  ├── service/
  │   └── mod.rs
  ├── adapters/
  │   └── mod.rs
  ├── storage/
  │   └── mod.rs
  ├── auth/
  │   └── mod.rs
  ├── lib.rs
  └── main.rs





Domain Logic

Define Core Data Models (Priority: High, Effort: ~3 hours)

Create structs for Adapter, Solver, Quote, and Intent with serde for JSON serialization.
Implement in-memory storage using DashMap for solvers and quotes with TTL for quotes.
Example:use serde::{Deserialize, Serialize};
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
struct Solver {
    solver_id: String,
    adapter_id: String,
    endpoint: String,
    timeout_ms: u64,
    status: String,
}

#[derive(Serialize, Deserialize)]
struct Quote {
    quote_id: String,
    solver_id: String,
    response: serde_json::Value,
}

struct Storage {
    solvers: Arc<DashMap<String, Solver>>,
    quotes: Arc<DashMap<String, Quote>>,
}




Implement Solver Adapter Trait (Priority: High, Effort: ~4 hours)

Define an async-trait for SolverAdapter with methods like get_quote and submit_intent.
Create a default OIF adapter implementation for HTTP-based solvers.
Example:use async_trait::async_trait;

#[async_trait]
trait SolverAdapter {
    async fn get_quote(&self, request: QuoteRequest) -> Result<Quote, Error>;
    async fn submit_intent(&self, intent: Intent) -> Result<IntentResponse, Error>;
}

struct OifAdapter {
    client: reqwest::Client,
    endpoint: String,
}

#[async_trait]
impl SolverAdapter for OifAdapter {
    async fn get_quote(&self, request: QuoteRequest) -> Result<Quote, Error> {
        // HTTP request to solver endpoint
        Ok(Quote { /* ... */ })
    }
    // ...
}




Implement Quote Aggregation Logic (Priority: High, Effort: ~5 hours)

Create a service layer to fetch quotes concurrently from solvers using tokio::spawn and tokio::time::timeout.
Normalize and aggregate responses, handling timeouts and partial failures.
Example:use tokio::time::{timeout, Duration};
use futures::future::join_all;

async fn fetch_quotes(solvers: Vec<Solver>, request: QuoteRequest) -> Vec<Quote> {
    let tasks = solvers.into_iter().map(|solver| {
        tokio::spawn(async move {
            let adapter = OifAdapter::new(solver.endpoint);
            timeout(Duration::from_millis(solver.timeout_ms), adapter.get_quote(request)).await
        })
    });
    let results = join_all(tasks).await;
    results.into_iter().filter_map(|r| r.ok().and_then(|r| r.ok())).collect()
}




Set Up REST API Endpoints (Priority: High, Effort: ~4 hours)

Implement /v1/quotes (POST) to trigger quote aggregation and return normalized JSON.
Implement /v1/intents (POST) to submit intents with quoteId or quoteResponse.
Implement /v1/intents/:id (GET) to query intent status.
Example:use axum::{routing::{get, post}, Router, extract::Path};

async fn post_quotes() -> axum::Json<Vec<Quote>> {
    // Call fetch_quotes
    axum::Json(vec![])
}

fn app() -> Router {
    Router::new()
        .route("/v1/quotes", post(post_quotes))
        .route("/v1/intents", post(submit_intent))
        .route("/v1/intents/:id", get(get_intent))
}




Implement IP-Based Rate Limiting (Priority: Medium, Effort: ~2 hours)

Use tower-http::limit or a custom tower::Service to enforce IP-based rate limiting.
Configure limits via environment variables (e.g., requests per minute per IP).
Example:use tower_http::limit::RateLimitLayer;
use std::time::Duration;

let app = Router::new()
    .route("/v1/quotes", post(post_quotes))
    .layer(RateLimitLayer::new(100, Duration::from_secs(60)));




Implement Builder Pattern (Priority: Medium, Effort: ~3 hours)

Create an AggregatorBuilder to configure solvers, adapters, storage, and auth programmatically.
Support methods like with_adapter, with_solver, with_storage, start.
Example:struct AggregatorBuilder {
    adapters: Vec<Box<dyn SolverAdapter>>,
    solvers: Vec<Solver>,
    storage: Box<dyn Storage>,
}

impl AggregatorBuilder {
    fn new() -> Self { /* ... */ }
    fn with_adapter(self, adapter: impl SolverAdapter + 'static) -> Self { /* ... */ }
    fn with_solver(self, solver: Solver) -> Self { /* ... */ }
    async fn start(self) -> Router { /* ... */ }
}




Add OpenAPI Support (Priority: Medium, Effort: ~3 hours)

Generate an OpenAPI spec for /v1/quotes, /v1/intents, and /v1/intents/:id using utoipa or similar.
Serve spec at /openapi.json and integrate with Swagger UI or Redoc.
Example:use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(post_quotes))]
struct ApiDoc;

fn app() -> Router {
    Router::new()
        .route("/openapi.json", get(|| async { axum::Json(ApiDoc::openapi()) }))
}




Add Structured Logging (Priority: Medium, Effort: ~2 hours)

Enhance logging with tracing to capture request details, solver response times, and errors.
Log timeouts, failed solver requests, and aggregation results.
Example:use tracing::info;

async fn post_quotes() -> axum::Json<Vec<Quote>> {
    info!("Received quote request");
    // ...
}




Support Stateless Intent Submission (Priority: Medium, Effort: ~2 hours)

Modify /v1/intents to accept a quoteResponse field in the request body for stateless validation.
Validate quoteResponse against solver metadata or signatures.
Example:#[derive(Deserialize)]
struct SubmitIntentRequest {
    quote_id: Option<String>,
    quote_response: Option<serde_json::Value>,
}




Add Solver Registration via Config (Priority: Low, Effort: ~2 hours)

Allow solver registration via JSON or ENV config files at startup.
Example config:{
  "solvers": [
    {
      "solver_id": "lifi-mainnet",
      "adapter_id": "lifi-v1",
      "endpoint": "https://api.lifi.com/mainnet",
      "timeout_ms": 2000
    }
  ]
}




Implement Basic Admin API (Priority: Low, Effort: ~4 hours)

Add /admin/solvers (GET, POST) for listing and registering solvers dynamically.
Secure with API key or IP whitelist.
Example:async fn list_solvers() -> axum::Json<Vec<Solver>> {
    // Fetch from storage
    axum::Json(vec![])
}





Notes

Prioritization Rationale:
High-priority tasks (1–8) focus on setting up a functional MVP with core domain logic (quote aggregation, REST API, solver adapters).
Medium-priority tasks (9–13) add critical features like rate limiting, OpenAPI, and stateless design for production readiness.
Low-priority tasks (14–15) are Phase 2 or optional features that enhance extensibility but aren’t critical for the MVP.


Effort Estimates: Based on a single developer familiar with Rust and Axum. Adjust for team size or experience.
Dependencies: Ensure Cargo.toml includes:[dependencies]
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
reqwest = { version = "0.12", features = ["json"] }
tower-http = { version = "0.5", features = ["limit"] }
async-trait = "0.1"
dashmap = "6.0"
tracing = "0.1"
utoipa = { version = "4.0", optional = true }


