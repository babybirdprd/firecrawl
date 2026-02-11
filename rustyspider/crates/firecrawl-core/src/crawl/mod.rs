use deadpool_redis::Pool;
use deadpool_redis::redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use crate::scraper::{ScrapeOptions, ScrapeResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlConfig {
    pub id: String,
    pub team_id: String,
    pub base_url: String,
    pub scrape_options: ScrapeOptions,
    pub max_depth: u32,
    pub limit: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrawlStatus {
    pub id: String,
    pub status: String,
    pub total: u32,
    pub completed: u32,
    pub active: u32,
}

pub struct CrawlManager {
    pool: Pool,
}

impl CrawlManager {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn save_config(&self, config: &CrawlConfig) -> anyhow::Result<()> {
        let mut conn = self.pool.get().await?;
        let key = format!("firecrawl:crawl:{}:config", config.id);
        let json = serde_json::to_string(config)?;
        let _: () = conn.set(key, json).await?;
        Ok(())
    }

    pub async fn get_config(&self, id: &str) -> anyhow::Result<Option<CrawlConfig>> {
        let mut conn = self.pool.get().await?;
        let key = format!("firecrawl:crawl:{}:config", id);
        let json: Option<String> = conn.get(key).await?;
        if let Some(json) = json {
            Ok(Some(serde_json::from_str(&json)?))
        } else {
            Ok(None)
        }
    }

    pub async fn set_robots_txt(&self, crawl_id: &str, robots_txt: &str) -> anyhow::Result<()> {
        let mut conn = self.pool.get().await?;
        let key = format!("firecrawl:crawl:{}:robots", crawl_id);
        let _: () = conn.set(key, robots_txt).await?;
        Ok(())
    }

    pub async fn get_robots_txt(&self, crawl_id: &str) -> anyhow::Result<Option<String>> {
        let mut conn = self.pool.get().await?;
        let key = format!("firecrawl:crawl:{}:robots", crawl_id);
        let robots_txt: Option<String> = conn.get(key).await?;
        Ok(robots_txt)
    }

    pub async fn lock_url(&self, crawl_id: &str, url: &str) -> anyhow::Result<bool> {
        let mut conn = self.pool.get().await?;
        let key = format!("firecrawl:crawl:{}:visited", crawl_id);
        // SADD returns the number of elements that were added to the set,
        // not including all the elements already present in the set.
        let added: i32 = conn.sadd(key, url).await?;
        Ok(added == 1)
    }

    pub async fn increment_count(&self, crawl_id: &str) -> anyhow::Result<u32> {
        let mut conn = self.pool.get().await?;
        let key = format!("firecrawl:crawl:{}:count", crawl_id);
        let count: u32 = conn.incr(key, 1).await?;
        Ok(count)
    }

    pub async fn get_count(&self, crawl_id: &str) -> anyhow::Result<u32> {
        let mut conn = self.pool.get().await?;
        let key = format!("firecrawl:crawl:{}:count", crawl_id);
        let count: Option<u32> = conn.get(key).await?;
        Ok(count.unwrap_or(0))
    }

    pub async fn increment_active_jobs(&self, crawl_id: &str) -> anyhow::Result<u32> {
        let mut conn = self.pool.get().await?;
        let key = format!("firecrawl:crawl:{}:active", crawl_id);
        let count: u32 = conn.incr(key, 1).await?;
        Ok(count)
    }

    pub async fn decrement_active_jobs(&self, crawl_id: &str) -> anyhow::Result<u32> {
        let mut conn = self.pool.get().await?;
        let key = format!("firecrawl:crawl:{}:active", crawl_id);
        let count: u32 = conn.decr(key, 1).await?;
        Ok(count)
    }

    pub async fn get_active_count(&self, crawl_id: &str) -> anyhow::Result<u32> {
        let mut conn = self.pool.get().await?;
        let key = format!("firecrawl:crawl:{}:active", crawl_id);
        let count: Option<u32> = conn.get(key).await?;
        Ok(count.unwrap_or(0))
    }

    pub async fn add_result(&self, crawl_id: &str, result: &ScrapeResult) -> anyhow::Result<()> {
        let mut conn = self.pool.get().await?;
        let key = format!("firecrawl:crawl:{}:results", crawl_id);
        let json = serde_json::to_string(result)?;
        let _: () = conn.rpush(key, json).await?;
        Ok(())
    }

    pub async fn get_results(&self, crawl_id: &str) -> anyhow::Result<Vec<ScrapeResult>> {
        let mut conn = self.pool.get().await?;
        let key = format!("firecrawl:crawl:{}:results", crawl_id);
        let results_json: Vec<String> = conn.lrange(key, 0, -1).await?;
        let mut results = Vec::with_capacity(results_json.len());
        for json in results_json {
            results.push(serde_json::from_str(&json)?);
        }
        Ok(results)
    }

    pub async fn get_status(&self, crawl_id: &str) -> anyhow::Result<Option<CrawlStatus>> {
        let config = self.get_config(crawl_id).await?;
        if config.is_none() {
            return Ok(None);
        }

        let total = self.get_count(crawl_id).await?;
        let active = self.get_active_count(crawl_id).await?;

        // We can estimate completed by results count
        let mut conn = self.pool.get().await?;
        let results_key = format!("firecrawl:crawl:{}:results", crawl_id);
        let completed: u32 = conn.llen(results_key).await?;

        let status = if active == 0 {
            "completed".to_string()
        } else {
            "scraping".to_string()
        };

        Ok(Some(CrawlStatus {
            id: crawl_id.to_string(),
            status,
            total,
            completed,
            active,
        }))
    }
}
