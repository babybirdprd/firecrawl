use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use firecrawl_core::scraper::service::ScrapeService;
use firecrawl_core::scraper::{ScrapeOptions, ScrapeResult};
use serde::Serialize;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "firecrawl_api=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let scrape_service = Arc::new(
        ScrapeService::new()
            .await
            .expect("Failed to initialize ScrapeService"),
    );

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/scrape", post(scrape))
        .with_state(scrape_service);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "OK"
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScrapeResponse {
    success: bool,
    data: Option<ScrapeResult>,
    error: Option<String>,
}

async fn scrape(
    State(service): State<Arc<ScrapeService>>,
    Json(options): Json<ScrapeOptions>,
) -> Json<ScrapeResponse> {
    match service.scrape(options).await {
        Ok(result) => Json(ScrapeResponse {
            success: true,
            data: Some(result),
            error: None,
        }),
        Err(e) => Json(ScrapeResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        }),
    }
}
