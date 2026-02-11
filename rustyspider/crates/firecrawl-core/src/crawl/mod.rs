use deadpool_redis::Pool;
use deadpool_redis::redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use crate::scraper::ScrapeOptions;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlConfig {
    pub id: String,
    pub team_id: String,
    pub base_url: String,
    pub scrape_options: ScrapeOptions,
    pub max_depth: u32,
    pub limit: u32,
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
}
