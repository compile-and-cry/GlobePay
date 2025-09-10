-- Sessions table to coordinate desktop QR page with mobile submission
CREATE TABLE IF NOT EXISTS sessions (
    id uuid PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'pending',
    payment_id uuid,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_sessions_status_created_at
    ON sessions (status, created_at DESC);

