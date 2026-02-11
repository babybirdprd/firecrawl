use std::sync::Arc;
use tokio::time::{sleep, Duration};
use crate::queue::{Queue, Job, JobPayload, JobStatus, ScrapeJobData, KickoffJobData, KickoffSitemapJobData};
use crate::scraper::service::ScrapeService;
use crate::scraper::ScrapeResult;
use crate::crawl::{CrawlManager, CrawlConfig};
use crate::html::extract_links;
use crate::crawler::{FilterLinksCall, filter_links};
use url::Url;

use tokio::sync::Semaphore;

pub struct Worker {
    queue: Arc<dyn Queue>,
    scrape_service: Arc<ScrapeService>,
    crawl_manager: Arc<CrawlManager>,
    semaphore: Arc<Semaphore>,
}

impl Worker {
    pub fn new(
        queue: Arc<dyn Queue>,
        scrape_service: Arc<ScrapeService>,
        crawl_manager: Arc<CrawlManager>,
        max_concurrency: usize,
    ) -> Self {
        Self {
            queue,
            scrape_service,
            crawl_manager,
            semaphore: Arc::new(Semaphore::new(max_concurrency)),
        }
    }

    pub async fn run(self: Arc<Self>) -> anyhow::Result<()> {
        tracing::info!("Worker started");
        loop {
            let permit = self.semaphore.clone().acquire_owned().await?;
            let worker = self.clone();

            match self.queue.pop().await {
                Ok(Some(job)) => {
                    tokio::spawn(async move {
                        let _permit = permit;
                        if let Err(e) = worker.handle_job_full_cycle(job).await {
                            tracing::error!("Error in job task: {}", e);
                        }
                    });
                }
                Ok(None) => {
                    // Permit dropped here
                }
                Err(e) => {
                    tracing::error!("Error popping job from queue: {}", e);
                    // Permit dropped here
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn handle_job_full_cycle(&self, job: Job<JobPayload>) -> anyhow::Result<()> {
        let job_id = job.id;
        let crawl_id = job.payload.crawl_id().map(|s| s.to_string());
        tracing::info!("Processing job: {:?} ({:?})", job_id, job.payload);

        match self.process_job(job).await {
            Ok(_) => {
                tracing::info!("Job completed: {:?}", job_id);
                if let Err(e) = self.queue.update_status(job_id, JobStatus::Completed).await {
                    tracing::error!("Failed to update job status for {:?}: {}", job_id, e);
                }
                if let Err(e) = self.queue.ack(job_id).await {
                    tracing::error!("Failed to ack job {:?}: {}", job_id, e);
                }
            }
            Err(e) => {
                tracing::error!("Error processing job {:?}: {}", job_id, e);
                if let Err(update_err) = self.queue.update_status(job_id, JobStatus::Failed(e.to_string())).await {
                    tracing::error!("Failed to update job status for {:?} to failed: {}", job_id, update_err);
                }
                if let Err(e) = self.queue.ack(job_id).await {
                    tracing::error!("Failed to ack job {:?} after failure: {}", job_id, e);
                }
            }
        }

        if let Some(cid) = crawl_id {
            if let Err(e) = self.crawl_manager.decrement_active_jobs(&cid).await {
                tracing::error!("Failed to decrement active jobs for {}: {}", cid, e);
            }
        }

        Ok(())
    }

    async fn push_job(&self, payload: JobPayload) -> anyhow::Result<uuid::Uuid> {
        if let Some(crawl_id) = payload.crawl_id() {
            self.crawl_manager.increment_active_jobs(crawl_id).await?;
        }
        self.queue.push(payload).await
    }

    async fn process_job(&self, job: Job<JobPayload>) -> anyhow::Result<()> {
        match job.payload {
            JobPayload::Scrape(data) => self.process_scrape(data).await,
            JobPayload::Kickoff(data) => self.process_kickoff(data).await,
            JobPayload::KickoffSitemap(data) => self.process_kickoff_sitemap(data).await,
        }
    }

    async fn process_kickoff(&self, data: KickoffJobData) -> anyhow::Result<()> {
        tracing::info!("Kicking off crawl for URL: {}", data.url);

        // 1. Save crawl config
        let config = CrawlConfig {
            id: data.crawl_id.clone(),
            team_id: data.team_id.clone(),
            base_url: data.url.clone(),
            scrape_options: data.scrape_options.clone(),
            max_depth: 10, // Default for now
            limit: 1000, // Default for now
        };
        self.crawl_manager.save_config(&config).await?;

        // 2. Lock and enqueue initial URL
        if self.crawl_manager.lock_url(&data.crawl_id, &data.url).await? {
            self.push_job(JobPayload::Scrape(ScrapeJobData {
                url: data.url.clone(),
                options: data.scrape_options.clone(),
                team_id: data.team_id.clone(),
                crawl_id: Some(data.crawl_id.clone()),
                is_crawl_source: true,
            })).await?;
        }

        // 3. Sitemap discovery (basic for now)
        let sitemap_url = if data.url.ends_with("/") {
            format!("{}sitemap.xml", data.url)
        } else {
            let mut u = Url::parse(&data.url)?;
            u.set_path("/sitemap.xml");
            u.to_string()
        };

        self.push_job(JobPayload::KickoffSitemap(KickoffSitemapJobData {
            sitemap_url,
            team_id: data.team_id.clone(),
            crawl_id: data.crawl_id.clone(),
        })).await?;

        // 4. Robots.txt discovery - always from domain root
        let robots_url = {
            let mut u = Url::parse(&data.url)?;
            u.set_path("/robots.txt");
            u.to_string()
        };

        let crawl_manager = self.crawl_manager.clone();
        let crawl_id = data.crawl_id.clone();
        tokio::spawn(async move {
            if let Ok(resp) = reqwest::get(&robots_url).await {
                if resp.status().is_success() {
                    if let Ok(robots_txt) = resp.text().await {
                        let _ = crawl_manager.set_robots_txt(&crawl_id, &robots_txt).await;
                    }
                }
            }
        });

        Ok(())
    }

    async fn process_kickoff_sitemap(&self, data: KickoffSitemapJobData) -> anyhow::Result<()> {
        tracing::info!("Processing sitemap: {}", data.sitemap_url);

        let config = self.crawl_manager.get_config(&data.crawl_id).await?
            .ok_or_else(|| anyhow::anyhow!("Crawl config not found for {}", data.crawl_id))?;

        // 1. Fetch sitemap
        let resp = reqwest::get(&data.sitemap_url).await?;
        if !resp.status().is_success() {
            anyhow::bail!("Failed to fetch sitemap {}: {}", data.sitemap_url, resp.status());
        }
        let xml = resp.text().await?;

        // 2. Parse sitemap
        let processing_result = crate::crawler::process_sitemap(xml).await?;

        // 3. Enqueue links/sitemaps
        for instruction in processing_result.instructions {
            match instruction.action.as_str() {
                "process" => {
                    for link in instruction.urls {
                        if self.crawl_manager.lock_url(&data.crawl_id, &link).await? {
                            let current_count = self.crawl_manager.get_count(&data.crawl_id).await?;
                            if current_count < config.limit {
                                self.crawl_manager.increment_count(&data.crawl_id).await?;
                                self.push_job(JobPayload::Scrape(ScrapeJobData {
                                    url: link,
                                    options: config.scrape_options.clone(),
                                    team_id: config.team_id.clone(),
                                    crawl_id: Some(data.crawl_id.clone()),
                                    is_crawl_source: false,
                                })).await?;
                            }
                        }
                    }
                }
                "recurse" => {
                    for sitemap_url in instruction.urls {
                        self.push_job(JobPayload::KickoffSitemap(KickoffSitemapJobData {
                            sitemap_url,
                            team_id: data.team_id.clone(),
                            crawl_id: data.crawl_id.clone(),
                        })).await?;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn process_scrape(&self, data: ScrapeJobData) -> anyhow::Result<()> {
        tracing::info!("Scraping URL: {}", data.url);

        let mut options = data.options.clone();
        options.url = data.url.clone();

        let result = self.scrape_service.scrape(options).await?;

        // Handle discovered links if it's a crawl
        if let Some(crawl_id) = &data.crawl_id {
            self.crawl_manager.add_result(crawl_id, &result).await?;
            self.handle_crawl_discovery(crawl_id, &data, &result).await?;
        }

        tracing::info!("Scrape successful for {}", data.url);

        Ok(())
    }

    async fn handle_crawl_discovery(&self, crawl_id: &str, _data: &ScrapeJobData, result: &ScrapeResult) -> anyhow::Result<()> {
        let config = self.crawl_manager.get_config(crawl_id).await?
            .ok_or_else(|| anyhow::anyhow!("Crawl config not found for {}", crawl_id))?;

        // 1. Extract links from HTML (if available)
        let html = result.html.clone().or_else(|| result.raw_html.clone());
        let discovered_links = extract_links(html).await?;

        // 2. Filter links
        let robots_txt = self.crawl_manager.get_robots_txt(crawl_id).await?.unwrap_or_default();

        let filter_call = FilterLinksCall {
            links: discovered_links,
            base_url: config.base_url.clone(),
            initial_url: config.base_url.clone(),
            max_depth: config.max_depth,
            limit: Some(config.limit as i64),
            excludes: vec![], // TODO: from config
            includes: vec![], // TODO: from config
            allow_backward_crawling: false,
            ignore_robots_txt: false,
            robots_txt,
            allow_external_content_links: false,
            allow_subdomains: false,
            regex_on_full_url: false,
        };

        let filter_result = filter_links(filter_call).await?;

        // 3. Enqueue new links
        for link in filter_result.links {
            if self.crawl_manager.lock_url(crawl_id, &link).await? {
                let current_count = self.crawl_manager.get_count(crawl_id).await?;
                if current_count < config.limit {
                    self.crawl_manager.increment_count(crawl_id).await?;
                    self.push_job(JobPayload::Scrape(ScrapeJobData {
                        url: link,
                        options: config.scrape_options.clone(),
                        team_id: config.team_id.clone(),
                        crawl_id: Some(crawl_id.to_string()),
                        is_crawl_source: false,
                    })).await?;
                }
            }
        }

        Ok(())
    }
}
