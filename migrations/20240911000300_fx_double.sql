-- Switch numeric to double precision for demo simplicity
ALTER TABLE payments
    ALTER COLUMN amount_inr TYPE DOUBLE PRECISION USING amount_inr::double precision,
    ALTER COLUMN source_amount TYPE DOUBLE PRECISION USING source_amount::double precision,
    ALTER COLUMN rate_to_inr TYPE DOUBLE PRECISION USING rate_to_inr::double precision;

ALTER TABLE fx_rates
    ALTER COLUMN rate TYPE DOUBLE PRECISION USING rate::double precision;

