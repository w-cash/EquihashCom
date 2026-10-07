//! Display helpers. Unknown values always render as "n/a".

pub const NA: &str = "n/a";

pub fn hashrate(v: Option<f64>, unit: &str) -> String {
    match v {
        Some(h) if h.is_finite() => {
            let base = unit.trim_end_matches("/s"); // "Sol" or "H"
            let (val, prefix) = scale(h);
            format!("{} {}{}/s", trim(val), prefix, base)
        }
        _ => NA.into(),
    }
}

fn scale(h: f64) -> (f64, &'static str) {
    let a = h.abs();
    if a >= 1e15 {
        (h / 1e15, "P")
    } else if a >= 1e12 {
        (h / 1e12, "T")
    } else if a >= 1e9 {
        (h / 1e9, "G")
    } else if a >= 1e6 {
        (h / 1e6, "M")
    } else if a >= 1e3 {
        (h / 1e3, "k")
    } else {
        (h, "")
    }
}

fn trim(v: f64) -> String {
    if v == 0.0 {
        "0".into()
    } else if v.abs() >= 100.0 {
        format!("{v:.0}")
    } else if v.abs() >= 10.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    }
}

pub fn int(v: Option<f64>) -> String {
    match v {
        Some(n) if n.is_finite() => group(n.round() as i128),
        _ => NA.into(),
    }
}

pub fn group(n: i128) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

pub fn compact(v: Option<f64>) -> String {
    match v {
        Some(n) if n.is_finite() && n != 0.0 && n.abs() < 1.0 => {
            // keep 3 significant digits for tiny values (e.g. low difficulty coins)
            let digits = (3 - n.abs().log10().ceil() as i32).clamp(0, 10) as usize;
            format!("{:.*}", digits, n)
        }
        Some(n) if n.is_finite() => {
            let (val, p) = scale(n);
            let p = match p {
                "k" => "K",
                "G" => "B",
                x => x,
            };
            format!("{}{}", trim(val), p)
        }
        _ => NA.into(),
    }
}

pub fn pct(v: Option<f64>) -> String {
    match v {
        Some(p) if p.is_finite() => {
            if p == 0.0 {
                "0%".into()
            } else if p < 0.01 {
                "<0.01%".into()
            } else if p < 10.0 {
                format!("{p:.2}%")
            } else {
                format!("{p:.1}%")
            }
        }
        _ => NA.into(),
    }
}

pub fn fee(range: Option<(f64, f64)>) -> String {
    match range {
        Some((lo, hi)) if (hi - lo).abs() < 1e-9 => format!("{}%", num_short(lo)),
        Some((lo, hi)) => format!("{}–{}%", num_short(lo), num_short(hi)),
        None => NA.into(),
    }
}

pub fn num_short(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

pub fn opt_num(v: Option<f64>) -> String {
    v.map(num_short).unwrap_or_else(|| NA.into())
}

/// USD price with sensible precision: $1,297 · $0.3122 · $0.0000293.
pub fn price(v: Option<f64>) -> String {
    match v {
        Some(p) if p.is_finite() && p >= 100.0 => format!("${}", group(p.round() as i128)),
        Some(p) if p.is_finite() && p >= 1.0 => format!("${p:.2}"),
        Some(p) if p.is_finite() && p > 0.0 => {
            let digits = (3 - p.log10().ceil() as i32).clamp(2, 10) as usize;
            format!("${:.*}", digits, p)
        }
        _ => NA.into(),
    }
}

pub fn seconds(v: Option<f64>) -> String {
    match v {
        Some(s) if s >= 120.0 => format!("{:.1} min", s / 60.0),
        Some(s) => format!("{} s", num_short((s * 10.0).round() / 10.0)),
        None => NA.into(),
    }
}

/// "2026-10-02T23:41:12.345Z" -> "2026-10-02 23:41 UTC"
pub fn utc(ts: Option<&str>) -> String {
    match ts.and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok()) {
        Some(d) => d
            .with_timezone(&chrono::Utc)
            .format("%Y-%m-%d %H:%M UTC")
            .to_string(),
        None => NA.into(),
    }
}

pub fn host(url: Option<&str>) -> String {
    url.map(|u| {
        u.trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("www.")
            .trim_end_matches('/')
            .to_string()
    })
    .unwrap_or_else(|| NA.into())
}

/// "2026-10-02T23:29:00Z" -> "2 Oct 2026" (UTC date).
pub fn date(ts: Option<&str>) -> String {
    match ts.and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok()) {
        Some(d) => d
            .with_timezone(&chrono::Utc)
            .format("%-d %b %Y")
            .to_string(),
        None => NA.into(),
    }
}

/// Age in words: "just now", "40 minutes", "10 hours", "3 days". None for a missing timestamp.
pub fn age(secs: Option<i64>) -> Option<String> {
    let s = secs?;
    Some(if s < 90 {
        "under a minute".into()
    } else if s < 5400 {
        format!("{} minutes", (s as f64 / 60.0).round() as i64)
    } else if s < 172_800 {
        format!("{} hours", (s as f64 / 3600.0).round() as i64)
    } else {
        format!("{} days", (s as f64 / 86400.0).round() as i64)
    })
}

/// The listed pools' total for a coin. "≥" when no network estimate exists (the pools' total is
/// then only a floor); "†" when any part of it is operator-reported. n/a stays n/a, zero stays 0.
pub fn reported(c: &crate::data::Coin) -> (String, bool) {
    let unit = c.network.unit.as_deref().unwrap_or("Sol/s");
    match c.reported.hashrate {
        None => (NA.into(), false),
        Some(h) => {
            let floor = c.network.hashrate.is_none() && h > 0.0;
            (
                format!(
                    "{}{}",
                    if floor { "≥" } else { "" },
                    hashrate(Some(h), unit)
                ),
                c.reported.operator_pools > 0,
            )
        }
    }
}

/// "16 million", "5,700", "140": a large factor in words, two significant figures.
pub fn factor(f: f64) -> String {
    let sig2 = |x: f64| {
        let mag = 10f64.powi(x.abs().log10().floor() as i32 - 1);
        (x / mag).round() * mag
    };
    let trimmed = |x: f64| {
        let t = format!("{x:.1}");
        t.trim_end_matches(".0").to_string()
    };
    if f >= 1e9 {
        format!("{} billion", trimmed(sig2(f) / 1e9))
    } else if f >= 1e6 {
        format!("{} million", trimmed(sig2(f) / 1e6))
    } else {
        group(sig2(f) as i128)
    }
}

/// Data-quality warning when a coin's network estimate and listed-pool total are extremely far
/// apart (see `data::DISCREPANCY_RATIO`). Neutral: it says the gap is there, not which side is wrong.
pub fn discrepancy_note(c: &crate::data::Coin) -> Option<String> {
    let f = c.discrepancy()?;
    let unit = c.network.unit.as_deref().unwrap_or("Sol/s");
    let (net, rep) = (
        hashrate(c.network.hashrate, unit),
        hashrate(c.reported.hashrate, unit),
    );
    let bigger = if c.network.hashrate > c.reported.hashrate {
        "network estimate"
    } else {
        "listed pools' total"
    };
    Some(format!(
        "The network estimate ({net}) and what the listed pools report ({rep}) differ by a factor of about {}, with the {bigger} the larger. Normal differences in measurement window or pool coverage don't explain a gap that size, so one of the sources is probably mismeasuring. Treat both figures, and the shares, with caution and check the coin's own explorer.",
        factor(f)
    ))
}

/// Footnote for a † on the listed pools' total.
pub fn operator_note(c: &crate::data::Coin) -> Option<String> {
    let r = &c.reported;
    if r.operator_pools == 0 {
        return None;
    }
    let pools = if r.operator_pools == 1 {
        "one listed pool".to_string()
    } else {
        format!("{} listed pools", r.operator_pools)
    };
    let all = r.operator_pools == r.positive_pools;
    let when = date(r.operator_observed_at.as_deref());
    Some(if all {
        format!("† Operator-reported; {pools}; verified {when}")
    } else {
        format!("† Includes an operator-reported figure from {pools}; verified {when}")
    })
}
