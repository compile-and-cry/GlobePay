use axum::{routing::{get, post}, Router, extract::{State, Query}, response::{Html, IntoResponse, Redirect, Response}, Form, Json};
use axum::http::StatusCode;
use tower_http::services::ServeDir;
use serde::Deserialize;
use tera::{Context};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::{AppState};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/pay", get(pay_form).post(create_payment))
        .route("/generate", post(create_payment))
        .route("/processing", get(processing))
        .route("/session_status", get(session_status))
        .route("/session_processing", post(session_processing))
        .route("/success", get(success))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
}

fn base_url() -> String {
    let port: u16 = std::env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(3000);
    if let Ok(raw) = std::env::var("PUBLIC_BASE_URL") {
        let candidate = if raw.starts_with("http://") || raw.starts_with("https://") { raw } else { format!("http://{}", raw) };
        if let Ok(u) = url::Url::parse(&candidate) {
            // Trust user-specified URL (e.g., ngrok https) and do not force-add port
            return u.as_str().trim_end_matches('/').to_string();
        }
    }
    if let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") {
        let _ = sock.connect("8.8.8.8:80");
        if let Ok(local) = sock.local_addr() {
            return format!("http://{}:{}", local.ip(), port);
        }
    }
    format!("http://localhost:{}", port)
}

async fn index(State(state): State<AppState>) -> Html<String> {
    // Create a session and show a QR that points to the payer form at /pay?sid=...
    let sid = state.db.create_session().await.expect("create session");
    let link = format!("{}/pay?sid={}", base_url(), sid);
    let qr = qr_data_url(&link);
    let mut ctx = Context::new();
    ctx.insert("scan_url", &link);
    ctx.insert("qr_data_url", &qr);
    ctx.insert("sid", &sid.to_string());
    let body = state.templates.render("scan_to_pay.html", &ctx).unwrap_or_else(|e| format!("Template error: {}", e));
    Html(body)
}

#[derive(Debug, Deserialize)]
struct PaymentForm {
    payer_name: String,
    upi_or_mobile: String,
    amount: f64,
    currency: String,
    note: Option<String>,
    sid: Option<String>,
}

fn normalize_upi(input: &str) -> String {
    let s = input.trim();
    if s.chars().all(|c| c.is_ascii_digit()) && (10..=12).contains(&s.len()) {
        format!("{}@upi", s)
    } else {
        s.to_string()
    }
}

fn upi_deeplink(pa: &str, pn: &str, amount: f64, note: Option<&str>) -> String {
    // Format with 2 decimal places for INR
    let am = format!("{:.2}", amount);
    let mut url = format!("upi://pay?pa={}&pn={}&am={}&cu=INR", urlencoding::encode(pa), urlencoding::encode(pn), am);
    if let Some(n) = note {
        if !n.trim().is_empty() {
            url.push_str(&format!("&tn={}", urlencoding::encode(n)));
        }
    }
    url
}

fn qr_data_url(data: &str) -> String {
    use qrcode::QrCode;
use image::{Luma, ImageBuffer};
use base64::Engine as _;
    let code = QrCode::new(data.as_bytes()).expect("QR generation failed");
    let image = code.render::<Luma<u8>>().min_dimensions(256, 256).build();
    // Encode to PNG and base64
    let mut buf = Vec::new();
    let dyn_img = image::DynamicImage::ImageLuma8(ImageBuffer::from_raw(image.width(), image.height(), image.into_raw()).unwrap());
    dyn_img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageOutputFormat::Png).unwrap();
    let b64 = base64::engine::general_purpose::STANDARD.encode(buf);
    format!("data:image/png;base64,{}", b64)
}

async fn pay_form(State(state): State<AppState>, Query(params): Query<std::collections::HashMap<String, String>>) -> Html<String> {
    let mut ctx = Context::new();
    if let Some(sid) = params.get("sid") { ctx.insert("sid", sid); }
    let body = state.templates.render("pay_form.html", &ctx).unwrap_or_else(|e| format!("Template error: {}", e));
    Html(body)
}

#[derive(Deserialize)]
struct WithSid {
    sid: Option<String>,
}

async fn create_payment(State(state): State<AppState>, Query(q): Query<WithSid>, Form(form): Form<PaymentForm>) -> Html<String> {
    let upi_id = normalize_upi(&form.upi_or_mobile);
    let src_ccy = form.currency.trim().to_uppercase();
    let (rate, rate_ts, provider) = if src_ccy == "INR" {
        (1.0, Some(Utc::now()), Some("static".to_string()))
    } else {
        match fetch_rate_to_inr(&src_ccy).await {
            Ok((r, ts, prov)) => (r, Some(ts), Some(prov)),
            Err(_) => {
                let r = fallback_rate(&src_ccy);
                (r, None, Some("fallback".to_string()))
            }
        }
    };

    // Convert to INR amount with 2 decimals
    let amount_inr = (form.amount * rate * 100.0).round() / 100.0;

    // Store fx rate record (best-effort)
    if src_ccy != "INR" {
        if let Some(ts) = rate_ts {
            let _ = state.db.insert_fx_rate(&src_ccy, "INR", rate, provider.as_deref(), Some(ts)).await;
        } else {
            let _ = state.db.insert_fx_rate(&src_ccy, "INR", rate, provider.as_deref(), None).await;
        }
    }

    let id = state
        .db
        .insert_payment(
            &form.payer_name,
            &upi_id,
            amount_inr,
            form.note.as_deref(),
            &src_ccy,
            form.amount,
            Some(rate),
            rate_ts,
        )
        .await
        .expect("DB insert failed");

    let sid_opt = q.sid.clone().or(form.sid.clone()).or_else(|| std::env::var("SID").ok());
    if let Some(sid_str) = sid_opt.clone() {
        if let Ok(sid) = Uuid::parse_str(&sid_str) {
            let _ = state.db.set_session_status(sid, "processing").await;
            let _ = state.db.attach_payment_to_session(sid, id).await;
        }
    }

    // Show a fun processing loader, then auto-redirect to success
    let mut ctx = Context::new();
    ctx.insert("id", &id.to_string());
    ctx.insert("amount_inr", &amount_inr);
    ctx.insert("source_amount", &form.amount);
    ctx.insert("source_currency", &src_ccy);
    if let Some(sid) = sid_opt { ctx.insert("sid", &sid); }
    let body = state.templates.render("processing.html", &ctx).unwrap_or_else(|e| format!("Template error: {}", e));
    Html(body)
}

async fn fetch_rate_to_inr(base: &str) -> anyhow::Result<(f64, DateTime<Utc>, String)> {
    if base == "INR" { return Ok((1.0, Utc::now(), "exchangerate.host-live".into())); }
    // Use exchangerate.host live endpoint with USD as the source; compute base->INR = USDINR / USDBASE
    let key = std::env::var("FX_API_KEY").unwrap_or_else(|_| "404823e62fd25735ff3f46242b2340f9".to_string());
    let base_up = base.to_uppercase();
    // Limit currencies to the two we need
    let currencies = if base_up == "USD" { "INR".to_string() } else { format!("INR,{}", base_up) };
    let url = format!("https://api.exchangerate.host/live?access_key={}&currencies={}", key, currencies);
    let resp = reqwest::Client::new().get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("rate http status {}", resp.status());
    }
    let v: serde_json::Value = resp.json().await?;
    if !v["success"].as_bool().unwrap_or(false) {
        anyhow::bail!("rate api returned error: {}", v);
    }
    let quotes = v["quotes"].as_object().ok_or_else(|| anyhow::anyhow!("missing quotes"))?;
    let usd_inr = quotes.get("USDINR").and_then(|x| x.as_f64()).ok_or_else(|| anyhow::anyhow!("missing USDINR"))?;
    let rate = if base_up == "USD" {
        usd_inr
    } else {
        let key = format!("USD{}", base_up);
        let usd_base = quotes.get(&key).and_then(|x| x.as_f64()).ok_or_else(|| anyhow::anyhow!("missing {}", key))?;
        if usd_base == 0.0 { anyhow::bail!("invalid 0 rate for {}", key); }
        usd_inr / usd_base
    };
    let ts = v["timestamp"].as_i64().unwrap_or_else(|| Utc::now().timestamp());
    let dt = chrono::DateTime::<Utc>::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now());
    Ok((rate, dt, "exchangerate.host-live".into()))
}

fn fallback_rate(base: &str) -> f64 {
    // Very rough demo fallback; update as needed.
    match base {
        "AED" => 22.5,
        "NPR" => 0.63,
        "BTN" => 1.0,
        "SGD" => 61.0,
        "MUR" => 1.7,
        "EUR" => 90.0,
        "LKR" => 0.25,
        _ => 1.0,
    }
}

async fn success(State(state): State<AppState>, axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>, headers: axum::http::HeaderMap) -> Response {
    let id_str = params.get("id").cloned().unwrap_or_default();
    let id = Uuid::parse_str(&id_str).ok();
    let mut amount_inr: Option<f64> = None;
    let mut source_amount: Option<f64> = None;
    let mut source_currency: Option<String> = None;
    let mut payer_name: Option<String> = None;

    if let Some(pid) = id {
        let _ = state.db.mark_success(pid).await;
        if let Ok(Some(p)) = state.db.get_payment(pid).await {
            amount_inr = Some(p.amount_inr);
            source_amount = Some(p.source_amount);
            source_currency = Some(p.source_currency);
            payer_name = Some(p.payer_name);
        }
    }
    if let Some(sid_str) = params.get("sid") {
        if let Ok(sid) = Uuid::parse_str(sid_str) {
            let _ = state.db.set_session_status(sid, "success").await;
        }
    }

    // If accessed via localhost (desktop), redirect to QR page instead of showing success
    if let Some(host) = headers.get(axum::http::header::HOST).and_then(|v| v.to_str().ok()) {
        if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
            return Redirect::to("/").into_response();
        }
    }
    let mut ctx = Context::new();
    if let Some(n) = payer_name { ctx.insert("payer_name", &n); }
    if let Some(a) = amount_inr { ctx.insert("amount_inr", &format!("{:.2}", a)); }
    if let Some(a) = source_amount { ctx.insert("source_amount", &format!("{:.2}", a)); }
    if let Some(c) = source_currency { ctx.insert("source_currency", &c); }
    let body = state.templates.render("success.html", &ctx).unwrap_or_else(|e| format!("Template error: {}", e));
    Html(body).into_response()
}

async fn processing(State(state): State<AppState>, Query(params): Query<std::collections::HashMap<String, String>>) -> Html<String> {
    let mut ctx = Context::new();
    if let Some(id) = params.get("id") { ctx.insert("id", id); }
    if let Some(sid) = params.get("sid") { ctx.insert("sid", sid); }
    let body = state.templates.render("processing.html", &ctx).unwrap_or_else(|e| format!("Template error: {}", e));
    Html(body)
}

async fn session_status(State(state): State<AppState>, Query(params): Query<std::collections::HashMap<String, String>>) -> impl IntoResponse {
    let sid_str = params.get("sid").cloned().unwrap_or_default();
    let status = if let Ok(sid) = Uuid::parse_str(&sid_str) {
        match state.db.get_session(sid).await.ok().flatten() {
            Some(s) => s.status,
            None => "not_found".into(),
        }
    } else { "invalid".into() };
    Json(serde_json::json!({"status": status}))
}

async fn session_processing(State(state): State<AppState>, Query(params): Query<std::collections::HashMap<String, String>>) -> impl IntoResponse {
    let sid_str = params.get("sid").cloned().unwrap_or_default();
    if let Ok(sid) = Uuid::parse_str(&sid_str) {
        let _ = state.db.set_session_status(sid, "processing").await;
        return StatusCode::NO_CONTENT;
    }
    StatusCode::BAD_REQUEST
}
