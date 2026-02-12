CREATE TABLE IF NOT EXISTS crawls (
    id TEXT PRIMARY KEY,
    team_id TEXT NOT NULL,
    base_url TEXT NOT NULL,
    config JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS scrape_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    crawl_id TEXT NOT NULL REFERENCES crawls(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_scrape_results_crawl_id ON scrape_results(crawl_id);
