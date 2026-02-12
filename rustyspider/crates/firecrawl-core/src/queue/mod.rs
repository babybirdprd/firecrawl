use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::scraper::ScrapeOptions;

pub mod redis;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job<T> {
    pub id: Uuid,
    pub payload: T,
    pub status: JobStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Active,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobPayload {
    Scrape(ScrapeJobData),
    Kickoff(KickoffJobData),
    KickoffSitemap(KickoffSitemapJobData),
}

impl JobPayload {
    pub fn crawl_id(&self) -> Option<&str> {
        match self {
            JobPayload::Scrape(data) => data.crawl_id.as_deref(),
            JobPayload::Kickoff(data) => Some(&data.crawl_id),
            JobPayload::KickoffSitemap(data) => Some(&data.crawl_id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeJobData {
    pub url: String,
    pub options: ScrapeOptions,
    pub team_id: String,
    pub crawl_id: Option<String>,
    pub is_crawl_source: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KickoffJobData {
    pub url: String,
    pub team_id: String,
    pub crawl_id: String,
    pub scrape_options: ScrapeOptions,
    pub limit: Option<u32>,
    pub max_depth: Option<u32>,
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KickoffSitemapJobData {
    pub sitemap_url: String,
    pub team_id: String,
    pub crawl_id: String,
}

#[async_trait]
pub trait Queue: Send + Sync {
    async fn push(&self, payload: JobPayload) -> anyhow::Result<Uuid>;
    async fn pop(&self) -> anyhow::Result<Option<Job<JobPayload>>>;
    async fn ack(&self, id: Uuid) -> anyhow::Result<()>;
    async fn get(&self, id: Uuid) -> anyhow::Result<Option<Job<JobPayload>>>;
    async fn update_status(&self, id: Uuid, status: JobStatus) -> anyhow::Result<()>;
}
