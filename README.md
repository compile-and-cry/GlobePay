GlobalPay — Postgres Demo

Quick Axum + Tera demo of a simulated UPI flow, backed by Postgres via sqlx. This is intended for demos only — there is no real UPI settlement. For production you must integrate with a licensed bank/PSP/PA and NPCI.

Run locally
- Prereq: Rust (1.75+), Postgres running
- Create a database and set `DATABASE_URL`, e.g. (Docker on port 5550):
  - `export DATABASE_URL=postgres://globalpay:gp_secret_5550@localhost:5550/globalpay`
- FX rates API key (exchangerate.host live):
  - `export FX_API_KEY=404823e62fd25735ff3f46242b2340f9`
- Optional: server port and base URL for QR
  - `export PORT=3000`
  - `export PUBLIC_BASE_URL=https://70a6bce83068.ngrok-free.app`  # or your LAN IP, or ngrok URL
- Recommended logs
  - `export RUST_LOG=info`

Environment exports (copy/paste)
```
export DATABASE_URL=postgres://globalpay:gp_secret_5550@localhost:5550/globalpay
export FX_API_KEY=404823e62fd25735ff3f46242b2340f9
export PORT=3000
export PUBLIC_BASE_URL=http://localhost:3000   # or https://<your-ngrok>.ngrok-free.app
export RUST_LOG=info
```
- Build and run:

  - `cargo run`
- Open: `http://localhost:3000/`

Notes
- Simulates success with a button; replace with bank/PSP webhook in production.
- Accepts UPI ID or mobile; mobile numbers get `@upi` appended for demo.
- QR generates a `upi://pay` deep-link; amount is integer INR for simplicity.
  - Conversion uses exchangerate.host `/live` endpoint (USD quotes). Base→INR computed as USDINR/USDBASE.

Architecture & Roadmap
- See `docs/design.puml` for PlantUML diagrams:
  - Architecture (components), Payment Flow (sequence), Deployment.
  - Roadmap (Phase 1–4: Demo → Pilot → Growth → Full Stack PSP).
  - Bottom Line summary of regulatory path and requirements.
  - Render with any PlantUML viewer or VS Code extension (e.g., "PlantUML").

AI (Demo)
- Fraud Risk: Lightweight 0–100 risk scoring with Low/Medium/High label and reasons (amount, cross‑border, UPI quality, keywords, time). Stored in `payments` and shown on processing/success.
- Explainer: `/ask` endpoint with a tiny keyword‑based FAQ that answers common questions (fees, FX, UPI vs. prod, env).
- Currency Optimizer: `/optimize_currency?amount=500` suggests the source currency that maximizes INR received for the same numeric amount, using fallback FX and demo fees.
- All AI features are demo‑grade. For production, use robust models, proper evaluation, and human review.

Migrations
- Located in `migrations/` and embedded via `sqlx::migrate!()`.
- They run automatically on startup when you `cargo run`.
- Optional manual control with sqlx-cli:
  - Install: `cargo install sqlx-cli --no-default-features --features native-tls,postgres`
  - Run: `sqlx migrate run`

Legal/Production
- UPI is regulated; production requires partnerships with bank/PSP/PA and NPCI compliance.
- Implement server-side order tracking, reconciliation, webhook signature verification, idempotency, retries.
- Do not ship demo code or flows (including the success button) into production.
