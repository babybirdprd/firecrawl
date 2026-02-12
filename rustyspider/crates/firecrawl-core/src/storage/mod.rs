use sqlx::postgres::PgPool;
use crate::crawl::CrawlConfig;
use crate::scraper::ScrapeResult;
use sqlx::Row;

pub struct Storage {
    pool: PgPool,
}

impl Storage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn save_crawl(&self, config: &CrawlConfig) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO crawls (id, team_id, base_url, config, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            ON CONFLICT (id) DO UPDATE SET config = $4
            "#,
        )
        .bind(&config.id)
        .bind(&config.team_id)
        .bind(&config.base_url)
        .bind(serde_json::to_value(config)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn save_scrape_result(&self, crawl_id: &str, result: &ScrapeResult) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO scrape_results (crawl_id, url, data, created_at)
            VALUES ($1, $2, $3, NOW())
            "#,
        )
        .bind(crawl_id)
        .bind(&result.url)
        .bind(serde_json::to_value(result)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_crawl_results(&self, crawl_id: &str) -> anyhow::Result<Vec<ScrapeResult>> {
        let rows = sqlx::query(
            r#"
            SELECT data FROM scrape_results WHERE crawl_id = $1 ORDER BY created_at ASC
            "#,
        )
        .bind(crawl_id)
        .fetch_all(&self.pool)
        .await?;

        let results = rows
            .into_iter()
            .map(|row| {
                let val: serde_json::Value = row.get("data");
                serde_json::from_value(val)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }
}
