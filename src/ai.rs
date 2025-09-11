use chrono::{Datelike, Timelike, Utc};

#[derive(Debug, Clone)]
pub struct RiskAssessment {
    pub score: i32,          // 0-100
    pub label: String,       // low/medium/high
    pub reasons: Vec<String> // short bullets
}

impl RiskAssessment {
    pub fn label_for(score: i32) -> &'static str {
        match score {
            s if s >= 70 => "high",
            s if s >= 40 => "medium",
            _ => "low",
        }
    }
}

// Lightweight heuristic "AI" risk scoring for demo purposes.
// Inputs are provided individually to decouple from HTTP types.
pub fn assess_risk(
    upi_id: &str,
    src_currency: &str,
    amount_inr: f64,
    note: Option<&str>,
) -> RiskAssessment {
    let mut score: f64 = 5.0; // baseline
    let mut reasons: Vec<String> = Vec::new();

    // Amount-based scaling (nonlinear, with caps)
    let a = amount_inr.max(0.0);
    if a > 50_000.0 { score += 20.0; reasons.push("high INR amount".into()); }
    if a > 200_000.0 { score += 18.0; reasons.push("very large ticket".into()); }
    if a > 500_000.0 { score += 18.0; reasons.push("extremely large ticket".into()); }

    // Cross-border bump when not INR
    if src_currency.to_uppercase() != "INR" {
        score += 12.0;
        reasons.push("cross-border remittance".into());
    }

    // UPI ID quality: prefer domain-like handles, penalize unusual suffixes
    let upi = upi_id.trim();
    if !upi.contains('@') { score += 15.0; reasons.push("invalid UPI format".into()); }
    if upi.contains('@') {
        let parts: Vec<&str> = upi.split('@').collect();
        if let Some(suffix) = parts.get(1) {
            let s = suffix.to_lowercase();
            let known = [
                "upi","oksbi","okhdfcbank","okicici","oksbi","ybl","ibl","axl","paytm","apl","sbi","rbi","axisbank","ibl"
            ];
            if !known.iter().any(|k| s.contains(k)) {
                score += 10.0; reasons.push("uncommon UPI handle".into());
            }
        }
        if parts.get(0).map_or(true, |p| p.is_empty()) { score += 8.0; reasons.push("empty UPI handle".into()); }
    }

    // Note-based simple NLP flags
    if let Some(n) = note {
        let nlow = n.to_lowercase();
        let flags = ["gift", "lottery", "refund", "crypto", "usdt", "investment", "urgent", "test"]; // demo
        if flags.iter().any(|k| nlow.contains(k)) {
            score += 10.0;
            reasons.push("message contains flagged keywords".into());
        }
    }

    // Time-of-day and weekend effects (demo only)
    let now = Utc::now();
    let hour = now.hour();
    let wd = now.weekday().number_from_monday(); // 1..=7
    if hour < 6 || hour >= 23 { score += 6.0; reasons.push("off-hours initiation".into()); }
    if wd >= 6 { score += 4.0; reasons.push("weekend initiation".into()); }

    // Clamp 0..100
    let score_i = score.max(0.0).min(100.0).round() as i32;
    let label = RiskAssessment::label_for(score_i).to_string();
    RiskAssessment { score: score_i, label, reasons }
}

// Very small FAQ-style answerer: keyword scoring over canned content.
pub fn answer_faq(question: &str) -> String {
    let q = question.to_lowercase();
    let entries: &[(&str, &str)] = &[
        (
            "fees",
            "Fees: INR payments have no fees in this demo. Cross-border includes a fixed ₹99 transfer fee and ₹25 platform fee, plus live FX.",
        ),
        (
            "fx rate",
            "FX: We fetch a live rate (or use a fallback) and compute base→INR. The rate and timestamp are stored per payment for transparency.",
        ),
        (
            "upi",
            "UPI: This is a simulated flow for demos. In production, integrate with a licensed bank/PSP/PA and verify webhooks before fulfillment.",
        ),
        (
            "production",
            "Production: Implement order tracking, reconciliation, webhook signature verification, idempotency and retries. Do not ship demo flows to prod.",
        ),
        (
            "env",
            "Environment: Set DATABASE_URL, FX_API_KEY, PORT, PUBLIC_BASE_URL, and RUST_LOG to run locally. See README for examples.",
        ),
        (
            "risk",
            "AI Risk: We compute a simple 0–100 risk score with Low/Medium/High label based on amount, cross-border, UPI handle quality, keywords, and time.",
        ),
    ];
    let mut best = (0usize, 0usize);
    for (i, (k, _)) in entries.iter().enumerate() {
        let mut score = 0usize;
        for token in k.split_whitespace() {
            if q.contains(token) { score += 1; }
        }
        if score > best.1 { best = (i, score); }
    }
    if best.1 == 0 {
        return "I can help with fees, FX, UPI demo vs. prod, env vars, and AI risk in this app.".to_string();
    }
    entries[best.0].1.to_string()
}

