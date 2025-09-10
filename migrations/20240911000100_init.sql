-- Initial schema for GlobalPay demo (Postgres)
CREATE TABLE IF NOT EXISTS payments (
    id uuid PRIMARY KEY,
    payer_name TEXT NOT NULL,
    upi_id TEXT NOT NULL,
    amount_inr BIGINT NOT NULL,
    note TEXT,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Useful index for lookups by status/time if you extend the demo
CREATE INDEX IF NOT EXISTS idx_payments_status_created_at
    ON payments (status, created_at);

