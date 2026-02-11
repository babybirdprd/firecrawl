use axum::{
    extract::{Json, State},
    routing::{get, post},
    Router,
};
use firecrawl_core::crawl::CrawlManager;
use firecrawl_core::queue::redis::RedisQueue;
use firecrawl_core::queue::{JobPayload, KickoffJobData, Queue};
use firecrawl_core::scraper::service::ScrapeService;
use firecrawl_core::scraper::{ScrapeOptions, ScrapeResult};
use firecrawl_core::worker::Worker;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "firecrawl_api=debug,firecrawl_core=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".into());

    let scrape_service = Arc::new(
        ScrapeService::new()
            .await
            .expect("Failed to initialize ScrapeService"),
    );

    // Create a single shared Redis connection pool
    let mut cfg = deadpool_redis::Config::default();
    cfg.url = Some(redis_url);
    let pool = cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1)).expect("Failed to create Redis pool");

    let queue = Arc::new(
        RedisQueue::new(pool.clone()),
    );

    let crawl_manager = Arc::new(CrawlManager::new(pool));

    let app_state = Arc::new(AppState {
        scrape_service: scrape_service.clone(),
        queue: queue.clone(),
        crawl_manager: crawl_manager.clone(),
    });

    // Start background worker
    let worker = Worker::new(queue, scrape_service, crawl_manager);
    tokio::spawn(async move {
        if let Err(e) = worker.run().await {
            tracing::error!("Worker error: {}", e);
        }
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/scrape", post(scrape))
        .route("/v1/crawl", post(crawl))
        .with_state(app_state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "OK"
}

struct AppState {
    scrape_service: Arc<ScrapeService>,
    queue: Arc<dyn Queue>,
    crawl_manager: Arc<CrawlManager>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScrapeResponse {
    success: bool,
    data: Option<ScrapeResult>,
    error: Option<String>,
}

async fn scrape(
    State(state): State<Arc<AppState>>,
    Json(options): Json<ScrapeOptions>,
) -> Json<ScrapeResponse> {
    match state.scrape_service.scrape(options).await {
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

#[derive(Deserialize)]
struct CrawlRequest {
    url: String,
    #[serde(default)]
    scrape_options: ScrapeOptions,
}

#[derive(Serialize)]
struct CrawlResponse {
    success: bool,
    id: Option<String>,
    url: Option<String>,
    error: Option<String>,
}

async fn crawl(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CrawlRequest>,
) -> Json<CrawlResponse> {
    let crawl_id = uuid::Uuid::now_v7().to_string();
    let team_id = "default".to_string(); // In a real app, this would come from auth

    let payload = JobPayload::Kickoff(KickoffJobData {
        url: req.url.clone(),
        team_id,
        crawl_id: crawl_id.clone(),
        scrape_options: req.scrape_options,
    });

    match state.queue.push(payload).await {
        Ok(_) => Json(CrawlResponse {
            success: true,
            id: Some(crawl_id.clone()),
            url: Some(format!("/v1/crawl/{}", crawl_id)), // Placeholder
            error: None,
        }),
        Err(e) => Json(CrawlResponse {
            success: false,
            id: None,
            url: None,
            error: Some(e.to_string()),
        }),
    }
}
