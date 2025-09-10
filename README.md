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
  - `export PUBLIC_BASE_URL=http://localhost:3000`  # or your LAN IP, or ngrok URL
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
