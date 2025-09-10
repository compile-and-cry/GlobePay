mod routes;
mod db;

use axum::{Router};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use tera::Tera;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct AppState {
    pub templates: Tera,
    pub db: db::Db,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let templates = Tera::new("templates/**/*")?;
    let db = db::Db::connect_from_env().await?;
    db.migrate().await?;

    let state = AppState { templates, db };

    let app: Router = routes::router(state);

    let port: u16 = std::env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(3000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let public_base = compute_public_base(port);
    tracing::info!(%port, public_base = %public_base, "Starting GlobalPay on 0.0.0.0");
    tracing::info!("Open desktop QR page: {}/", public_base);
    tracing::info!("Mobile will open: {}/pay?sid=<session>", public_base);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn detect_lan_ip() -> Option<IpAddr> {
    // Use a UDP connect trick to discover the chosen outbound interface IP
    let sock = std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    // No packets are sent until we write; this just sets the local addr
    let _ = sock.connect("8.8.8.8:80");
    sock.local_addr().ok().map(|sa| sa.ip())
}

fn compute_public_base(port: u16) -> String {
    if let Ok(raw) = std::env::var("PUBLIC_BASE_URL") {
        // Ensure scheme and port are present
        let candidate = if raw.starts_with("http://") || raw.starts_with("https://") {
            raw
        } else {
            format!("http://{}", raw)
        };
        if let Ok(u) = url::Url::parse(&candidate) {
            // Trust user-specified URL as-is (do not force-add port)
            return u.as_str().trim_end_matches('/').to_string();
        }
    }
    let ip = detect_lan_ip().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
    format!("http://{}:{}", ip, port)
}
