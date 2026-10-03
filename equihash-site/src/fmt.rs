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
        Some(d) => d.with_timezone(&chrono::Utc).format("%Y-%m-%d %H:%M UTC").to_string(),
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
