-- FX support: store source currency/amount and rate, change INR type
ALTER TABLE payments
    ALTER COLUMN amount_inr TYPE NUMERIC(18,2) USING amount_inr::numeric;

ALTER TABLE payments
    ADD COLUMN IF NOT EXISTS source_currency TEXT NOT NULL DEFAULT 'INR',
    ADD COLUMN IF NOT EXISTS source_amount NUMERIC(18,2) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS rate_to_inr NUMERIC(18,8),
    ADD COLUMN IF NOT EXISTS rate_timestamp TIMESTAMPTZ;

CREATE TABLE IF NOT EXISTS fx_rates (
    id uuid PRIMARY KEY,
    base_currency TEXT NOT NULL,
    quote_currency TEXT NOT NULL,
    rate NUMERIC(18,8) NOT NULL,
    provider TEXT,
    fetched_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_fx_rates_base_quote_time
    ON fx_rates (base_currency, quote_currency, fetched_at DESC);

