use axum::{
    extract::{Json, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Router,
};
use firecrawl_core::crawl::CrawlManager;
use firecrawl_core::storage::Storage;
use firecrawl_core::queue::redis::RedisQueue;
use firecrawl_core::queue::{JobPayload, KickoffJobData, Queue};
use firecrawl_core::scraper::service::ScrapeService;
use firecrawl_core::scraper::{ScrapeOptions, ScrapeResult};
use firecrawl_core::worker::Worker;
use deadpool_redis::redis::AsyncCommands;
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
    let proxy_url = std::env::var("PROXY_SERVER").ok();

    let scrape_service = Arc::new(
        ScrapeService::new(proxy_url)
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

    let database_url = std::env::var("DATABASE_URL").ok();
    let storage = if let Some(url) = database_url {
        let pg_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await
            .expect("Failed to connect to Postgres");

        // Run migrations
        sqlx::migrate!("../../crates/firecrawl-core/migrations")
            .run(&pg_pool)
            .await
            .expect("Failed to run migrations");

        Some(Arc::new(Storage::new(pg_pool)))
    } else {
        None
    };

    let crawl_manager = Arc::new(CrawlManager::new(pool, storage));

    let app_state = Arc::new(AppState {
        scrape_service: scrape_service.clone(),
        queue: queue.clone(),
        crawl_manager: crawl_manager.clone(),
    });

    // Start background worker
    let worker = Arc::new(Worker::new(queue, scrape_service, crawl_manager, 10));
    tokio::spawn(async move {
        if let Err(e) = worker.run().await {
            tracing::error!("Worker error: {}", e);
        }
    });

    let app = Router::new()
        .route("/health", get(health))
        .nest(
            "/v1",
            Router::new()
                .route("/scrape", post(scrape))
                .route("/crawl", post(crawl))
                .route("/crawl/:id", get(get_crawl_status))
                .layer(middleware::from_fn_with_state(app_state.clone(), rate_limit))
                .layer(middleware::from_fn(auth)),
        )
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

async fn rate_limit(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let api_key = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("anonymous");

    let mut conn = state
        .crawl_manager
        .pool()
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let key = format!("firecrawl:ratelimit:{}", api_key);

    // Simple fixed window rate limiting: 100 requests per minute
    let count: u32 = conn
        .incr(&key, 1)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if count == 1 {
        let _: () = conn
            .expire(&key, 60)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if count > 100 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(req).await)
}

async fn auth(req: Request, next: Next) -> Result<Response, StatusCode> {
    let api_key = std::env::var("API_KEY").ok();

    // If API_KEY is not set, allow all requests (for development)
    if api_key.is_none() {
        return Ok(next.run(req).await);
    }

    let api_key = api_key.unwrap();
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    if let Some(auth_header) = auth_header {
        if auth_header == format!("Bearer {}", api_key) {
            return Ok(next.run(req).await);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
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
#[serde(rename_all = "camelCase")]
struct CrawlRequest {
    url: String,
    #[serde(default)]
    scrape_options: ScrapeOptions,
    limit: Option<u32>,
    max_depth: Option<u32>,
    #[serde(default)]
    includes: Vec<String>,
    #[serde(default)]
    excludes: Vec<String>,
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
        limit: req.limit,
        max_depth: req.max_depth,
        includes: req.includes,
        excludes: req.excludes,
    });

    // Increment active jobs for the kickoff job
    if let Err(e) = state.crawl_manager.increment_active_jobs(&crawl_id).await {
         return Json(CrawlResponse {
            success: false,
            id: None,
            url: None,
            error: Some(format!("Failed to initialize crawl: {}", e)),
        });
    }

    match state.queue.push(payload).await {
        Ok(_) => Json(CrawlResponse {
            success: true,
            id: Some(crawl_id.clone()),
            url: Some(format!("/v1/crawl/{}", crawl_id)),
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CrawlStatusResponse {
    success: bool,
    status: String,
    completed: u32,
    total: u32,
    credits_used: u32,
    expires_at: String,
    data: Vec<ScrapeResult>,
    error: Option<String>,
}

async fn get_crawl_status(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<CrawlStatusResponse> {
    match state.crawl_manager.get_status(&id).await {
        Ok(Some(status)) => {
            let results = state.crawl_manager.get_results(&id).await.unwrap_or_default();
            Json(CrawlStatusResponse {
                success: true,
                status: status.status,
                completed: status.completed,
                total: status.total,
                credits_used: status.completed, // Placeholder
                expires_at: "".to_string(), // Placeholder
                data: results,
                error: None,
            })
        }
        Ok(None) => Json(CrawlStatusResponse {
            success: false,
            status: "not_found".to_string(),
            completed: 0,
            total: 0,
            credits_used: 0,
            expires_at: "".to_string(),
            data: vec![],
            error: Some("Crawl not found".to_string()),
        }),
        Err(e) => Json(CrawlStatusResponse {
            success: false,
            status: "error".to_string(),
            completed: 0,
            total: 0,
            credits_used: 0,
            expires_at: "".to_string(),
            data: vec![],
            error: Some(e.to_string()),
        }),
    }
}
