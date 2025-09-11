-- AI risk assessment columns for payments
ALTER TABLE payments
    ADD COLUMN IF NOT EXISTS risk_score INTEGER NOT NULL DEFAULT 5,
    ADD COLUMN IF NOT EXISTS risk_label TEXT NOT NULL DEFAULT 'low',
    ADD COLUMN IF NOT EXISTS risk_reasons TEXT;

CREATE INDEX IF NOT EXISTS idx_payments_risk
    ON payments (risk_score DESC, created_at DESC);

