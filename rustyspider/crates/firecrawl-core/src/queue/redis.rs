use async_trait::async_trait;
use deadpool_redis::Pool;
use deadpool_redis::redis::AsyncCommands;
use uuid::Uuid;
use crate::queue::{Queue, Job, JobPayload, JobStatus};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct RedisQueue {
    pool: Pool,
    queue_key: String,
    processing_key: String,
}

impl RedisQueue {
    pub fn new(pool: Pool) -> Self {
        Self {
            pool,
            queue_key: "firecrawl:jobs:queue".to_string(),
            processing_key: "firecrawl:jobs:processing".to_string(),
        }
    }

    fn job_key(&self, id: Uuid) -> String {
        format!("firecrawl:jobs:data:{}", id)
    }

    fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }
}

#[async_trait]
impl Queue for RedisQueue {
    async fn push(&self, payload: JobPayload) -> anyhow::Result<Uuid> {
        let mut conn = self.pool.get().await?;
        let id = Uuid::now_v7();
        let now = Self::now();

        let job = Job {
            id,
            payload,
            status: JobStatus::Pending,
            created_at: now,
            updated_at: now,
        };

        let job_json = serde_json::to_string(&job)?;

        // Store job data
        let _: () = conn.set(self.job_key(id), job_json).await?;
        // Push job ID to queue
        let _: () = conn.rpush(&self.queue_key, id.to_string()).await?;

        Ok(id)
    }

    async fn pop(&self) -> anyhow::Result<Option<Job<JobPayload>>> {
        let mut conn = self.pool.get().await?;

        // Atomic and blocking move from queue to processing
        let id_str: Option<String> = conn.brpoplpush(&self.queue_key, &self.processing_key, 1.0).await?;

        if let Some(id_str) = id_str {
            let id = Uuid::parse_str(&id_str)?;
            let job_json: Option<String> = conn.get(self.job_key(id)).await?;

            if let Some(job_json) = job_json {
                let mut job: Job<JobPayload> = serde_json::from_str(&job_json)?;
                job.status = JobStatus::Active;
                job.updated_at = Self::now();

                let job_json = serde_json::to_string(&job)?;
                let _: () = conn.set(self.job_key(id), job_json).await?;

                Ok(Some(job))
            } else {
                // Job data missing, but ID was in queue. Clean up.
                let _: () = conn.lrem(&self.processing_key, 0, &id_str).await?;
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn ack(&self, id: Uuid) -> anyhow::Result<()> {
        let mut conn = self.pool.get().await?;
        let _: () = conn.lrem(&self.processing_key, 0, id.to_string()).await?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> anyhow::Result<Option<Job<JobPayload>>> {
        let mut conn = self.pool.get().await?;
        let job_json: Option<String> = conn.get(self.job_key(id)).await?;

        if let Some(job_json) = job_json {
            Ok(Some(serde_json::from_str(&job_json)?))
        } else {
            Ok(None)
        }
    }

    async fn update_status(&self, id: Uuid, status: JobStatus) -> anyhow::Result<()> {
        let mut conn = self.pool.get().await?;
        let job_json: Option<String> = conn.get(self.job_key(id)).await?;

        if let Some(job_json) = job_json {
            let mut job: Job<JobPayload> = serde_json::from_str(&job_json)?;
            job.status = status;
            job.updated_at = Self::now();

            let job_json = serde_json::to_string(&job)?;
            let _: () = conn.set(self.job_key(id), job_json).await?;
        }

        Ok(())
    }
}
