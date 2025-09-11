use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct Db {
    pub pool: PgPool,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Payment {
    pub id: Uuid,
    pub payer_name: String,
    pub upi_id: String,
    pub amount_inr: f64,
    pub note: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub source_currency: String,
    pub source_amount: f64,
    pub rate_to_inr: Option<f64>,
    pub rate_timestamp: Option<DateTime<Utc>>,
    pub fee_transfer_inr: f64,
    pub fee_platform_inr: f64,
    pub fee_src_total: f64,
    pub total_inr: f64,
    pub total_src: f64,
    pub risk_score: Option<i32>,
    pub risk_label: Option<String>,
    pub risk_reasons: Option<String>,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub status: String,
    pub payment_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl Db {
    pub async fn connect_from_env() -> anyhow::Result<Self> {
        let url = std::env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL not set. Example: postgres://user:pass@localhost:5432/globalpay"))?;
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> anyhow::Result<()> {
        // Run embedded migrations from ./migrations
        sqlx::migrate!().run(&self.pool).await?;
        Ok(())
    }

    pub async fn insert_payment(
        &self,
        payer_name: &str,
        upi_id: &str,
        amount_inr: f64,
        note: Option<&str>,
        source_currency: &str,
        source_amount: f64,
        rate_to_inr: Option<f64>,
        rate_timestamp: Option<DateTime<Utc>>,
        fee_transfer_inr: f64,
        fee_platform_inr: f64,
        fee_src_total: f64,
        total_inr: f64,
        total_src: f64,
        risk_score: i32,
        risk_label: &str,
        risk_reasons: Option<&str>,
    ) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO payments (
                    id, payer_name, upi_id, amount_inr, note, status,
                    source_currency, source_amount, rate_to_inr, rate_timestamp,
                    fee_transfer_inr, fee_platform_inr, fee_src_total, total_inr, total_src,
                    risk_score, risk_label, risk_reasons
               ) VALUES (
                    $1,$2,$3,$4,$5,'pending',$6,$7,$8,$9,$10,$11,$12,$13,$14,
                    $15,$16,$17
               )"#,
        )
        .bind(id)
        .bind(payer_name)
        .bind(upi_id)
        .bind(amount_inr)
        .bind(note)
        .bind(source_currency)
        .bind(source_amount)
        .bind(rate_to_inr)
        .bind(rate_timestamp)
        .bind(fee_transfer_inr)
        .bind(fee_platform_inr)
        .bind(fee_src_total)
        .bind(total_inr)
        .bind(total_src)
        .bind(risk_score)
        .bind(risk_label)
        .bind(risk_reasons)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn mark_success(&self, id: Uuid) -> anyhow::Result<()> {
        sqlx::query("UPDATE payments SET status = 'success' WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_payment(&self, id: Uuid) -> anyhow::Result<Option<Payment>> {
        let rec = sqlx::query_as::<_, Payment>("SELECT * FROM payments WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(rec)
    }

    pub async fn insert_fx_rate(
        &self,
        base_currency: &str,
        quote_currency: &str,
        rate: f64,
        provider: Option<&str>,
        fetched_at: Option<DateTime<Utc>>,
    ) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO fx_rates (id, base_currency, quote_currency, rate, provider, fetched_at)
                VALUES ($1,$2,$3,$4,$5, COALESCE($6, now()))"#,
        )
        .bind(id)
        .bind(base_currency)
        .bind(quote_currency)
        .bind(rate)
        .bind(provider)
        .bind(fetched_at)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn create_session(&self) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO sessions (id) VALUES ($1)")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(id)
    }

    pub async fn set_session_status(&self, id: Uuid, status: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE sessions SET status = $2 WHERE id = $1")
            .bind(id)
            .bind(status)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn attach_payment_to_session(&self, id: Uuid, payment_id: Uuid) -> anyhow::Result<()> {
        sqlx::query("UPDATE sessions SET payment_id = $2 WHERE id = $1")
            .bind(id)
            .bind(payment_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_session(&self, id: Uuid) -> anyhow::Result<Option<Session>> {
        let rec = sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(rec)
    }
}
