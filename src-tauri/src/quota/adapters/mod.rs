pub mod codex;
pub mod deepseek;
pub mod gemini;
pub mod openrouter;

pub(crate) fn parse_rfc3339_seconds(s: &str) -> Option<i64> {
    let s = s.trim_end_matches('Z');
    let parts: Vec<&str> = s.split('T').collect();
    if parts.len() != 2 {
        return None;
    }
    let date_parts: Vec<i64> = parts[0].split('-').filter_map(|p| p.parse().ok()).collect();
    if date_parts.len() != 3 {
        return None;
    }
    let raw_time_parts: Vec<&str> = parts[1].split(':').collect();
    if raw_time_parts.len() < 3 {
        return None;
    }
    let hour: i64 = raw_time_parts[0].parse().ok()?;
    let min: i64 = raw_time_parts[1].parse().ok()?;
    let sec_str = raw_time_parts[2].split('.').next().unwrap_or(raw_time_parts[2]);
    let sec: i64 = sec_str.parse().ok()?;

    let y = date_parts[0];
    let m = date_parts[1];
    let d = date_parts[2];

    let mut days = (y - 1970) * 365 + (y - 1969) / 4;
    let month_days = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for i in 1..m {
        days += month_days[i as usize];
    }
    if m > 2 && (y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)) {
        days += 1;
    }
    days += d - 1;

    Some(days * 86400 + hour * 3600 + min * 60 + sec)
}
