# Rust Port Roadmap: Firecrawl (Single Binary)

This document outlines the architectural plan and roadmap for porting Firecrawl to a monolithic, single-binary Rust application.

## 🎯 Goal
Create a high-performance, single-binary replacement for the existing microservices-based Firecrawl architecture, leveraging Rust's safety, concurrency, and ecosystem.

## 🏗️ Architecture
The new architecture consolidates the API, Worker, and Scraper services into a unified application.

### Core Components
1.  **API Server (`axum`)**:
    -   Handles HTTP requests (`/scrape`, `/crawl`, etc.).
    -   Manages authentication and rate limiting.
    -   Dispatches jobs to the internal queue.
2.  **Job Queue (Internal/Redis)**:
    -   Manages asynchronous scraping tasks.
    -   Initially backed by Redis (for compatibility/robustness), but abstracted to allow future in-memory or DB-backed implementations.
3.  **Scraping Engine**:
    -   **Browser Engine**: Uses `chromiumoxide` (CDP) to control a local Headless Chrome instance. Replaces `playwright-service-ts`.
    -   **HTTP Engine**: Uses `reqwest` for lightweight, fast scraping when browser rendering is unnecessary.
4.  **Content Processor**:
    -   **HTML Parsing**: Uses `kuchikiki` (HTML5 parser) for DOM manipulation.
    -   **Markdown Conversion**: Custom implementation or library (e.g., `html2text`, `htmd`) to convert HTML to Markdown, optimized for LLM consumption.
    -   **Extraction**: Integration with LLM providers (OpenAI, etc.) for structured data extraction.
5.  **Storage (`sqlx`)**:
    -   PostgreSQL for persistent storage of job states, results, and user data.

## 🚀 Migration Strategy
The migration will be performed in phases, prioritizing core functionality.

### Phase 1: Foundation & API Skeleton
- [x] Initialize Cargo workspace.
- [x] Set up `axum` server with basic health check.
-   Implement configuration management (env vars).
- [x] Set up logging/tracing.

### Phase 2: Scraping Engine (The "Hard Part")
- [x] Implement `BrowserScraper` using `chromiumoxide`.
    - [x] Browser lifecycle management.
    - [x] Page navigation, content retrieval.
    - [x] Proxy support (via global configuration).
    - [x] Ad/Tracker blocking (network interception).
- [x] Implement `HttpScraper` using `reqwest`.

### Phase 3: Content Processing
- [x] Port HTML cleaning logic from `apps/api/native`.
-   Implement robust HTML-to-Markdown conversion.
-   Integrate with Scraping Engine.

### Phase 4: Job Queue & Worker
- [x] Implement job queue system (Redis-backed).
- [x] Create worker loop to consume jobs and execute scraping tasks.
- [x] Implement concurrency controls (semaphores/rate limiters).

### Phase 5: API Implementation
- [x] Implement `/scrape` endpoint (synchronous/async).
- [x] Implement `/crawl` endpoints (asynchronous with enhanced configuration).
-   Integrate Auth and Rate Limiting middleware.
-   Database integration for job persistence.

### Phase 6: Polish & Features
- [/] LLM Extraction logic (Trait-based extensible architecture implemented).
-   Webhook notifications.
- [/] Sitemap discovery (Enhanced discovery via `robots.txt` implemented).
-   Deep research features.

## 📦 Dependencies
-   **Web Framework**: `axum`
-   **Async Runtime**: `tokio`
-   **HTTP Client**: `reqwest`
-   **Browser Control**: `chromiumoxide`
-   **Database**: `sqlx` (Postgres)
-   **Redis**: `redis` / `deadpool-redis`
-   **Serialization**: `serde`, `serde_json`
-   **HTML Parsing**: `kuchikiki`, `scraper`
-   **Logging**: `tracing`, `tracing-subscriber`
-   **Config**: `config` or `dotenvy`
-   **Error Handling**: `thiserror`, `anyhow`

## 📝 Notes
-   **Single Binary**: The app will start both the API server and the background worker threads within the same process. CLI flags or env vars can control if it runs in "api-only", "worker-only", or "all" (default) mode.
-   **Browser**: The binary assumes a compatible Chrome/Chromium executable is available on the system path.
