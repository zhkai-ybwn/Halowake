use std::collections::VecDeque;

use super::ProjectProcessLogLine;

#[allow(dead_code)]
pub(super) fn detect_ports(lines: &VecDeque<ProjectProcessLogLine>) -> Vec<u16> {
    let mut ports = Vec::new();
    for line in lines.iter().rev() {
        append_detected_ports(&line.text, &mut ports);
        if ports.len() >= 4 {
            break;
        }
    }
    ports.sort_unstable();
    ports
}

#[allow(dead_code)]
pub(super) fn detect_urls(lines: &VecDeque<ProjectProcessLogLine>) -> Vec<String> {
    let mut urls = Vec::new();
    for line in lines.iter().rev() {
        append_detected_urls(&line.text, &mut urls);
    }
    urls
}

pub(super) fn append_detected_ports(text: &str, ports: &mut Vec<u16>) {
    let text = strip_ansi(text);
    let lower = text.to_lowercase();
    if !lower.contains(':') && !lower.contains("port") {
        return;
    }

    for token in text.split_whitespace() {
        let candidate = token
            .trim_matches(|ch: char| {
                matches!(
                    ch,
                    '(' | ')'
                        | '['
                        | ']'
                        | '<'
                        | '>'
                        | ','
                        | ';'
                        | '"'
                        | '\''
                        | '|'
                        | '`'
                        | '#'
                        | '!'
                )
            })
            .trim_end_matches('.')
            .trim_end_matches(':')
            .trim_end_matches(',');
        if let Some(port) = extract_port_from_host_port(candidate) {
            if !ports.contains(&port) {
                ports.push(port);
            }
        }
    }

    let words: Vec<&str> = lower.split_whitespace().collect();
    for (index, word) in words.iter().enumerate() {
        let cleaned = word.trim_matches(|ch: char| !ch.is_alphanumeric() && ch != ':' && ch != '=');
        if cleaned == "port" || cleaned == "port:" || cleaned == "port=" {
            if let Some(next_word) = words.get(index + 1) {
                let digits: String = next_word
                    .chars()
                    .take_while(|ch| ch.is_ascii_digit())
                    .collect();
                if let Ok(port) = digits.parse::<u16>() {
                    if (1024..=65535).contains(&port) && !ports.contains(&port) {
                        ports.push(port);
                    }
                }
            }
        } else if let Some(rest) = cleaned
            .strip_prefix("port:")
            .or_else(|| cleaned.strip_prefix("port="))
        {
            let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
            if let Ok(port) = digits.parse::<u16>() {
                if (1024..=65535).contains(&port) && !ports.contains(&port) {
                    ports.push(port);
                }
            }
        }
    }
}

pub(super) fn append_detected_urls(text: &str, urls: &mut Vec<String>) {
    if !text.contains("http://") && !text.contains("https://") {
        return;
    }
    let text = strip_ansi(text);
    for token in text.split_whitespace() {
        let candidate = token
            .trim_matches(|ch: char| {
                matches!(
                    ch,
                    '(' | ')'
                        | '['
                        | ']'
                        | '<'
                        | '>'
                        | ','
                        | ';'
                        | '"'
                        | '\''
                        | '|'
                        | '`'
                        | '#'
                        | '!'
                )
            })
            .trim_end_matches('.')
            .trim_end_matches(':')
            .trim_end_matches(',');
        if (candidate.starts_with("http://") || candidate.starts_with("https://"))
            && is_localhost_url(candidate)
            && !urls.iter().any(|url| url == candidate)
        {
            urls.push(candidate.to_string());
        }
    }
}

fn is_valid_local_host(host: &str) -> bool {
    let host = host.trim_matches('[').trim_matches(']');
    host == "localhost"
        || host == "127.0.0.1"
        || host == "0.0.0.0"
        || host == "::1"
        || host == "::"
        || host.ends_with(".local")
        || is_private_ip(host)
}

fn extract_port_from_host_port(token: &str) -> Option<u16> {
    let without_protocol = token
        .strip_prefix("https://")
        .or_else(|| token.strip_prefix("http://"))
        .unwrap_or(token);
    let host_and_port = without_protocol.split('/').next().unwrap_or("");
    let index = host_and_port.rfind(':')?;
    let host = &host_and_port[..index];
    let port_text = &host_and_port[index + 1..];
    if !is_valid_local_host(host) {
        return None;
    }
    let port = port_text.parse::<u16>().ok()?;
    (1024..=65535).contains(&port).then_some(port)
}

fn is_localhost_url(url: &str) -> bool {
    let without_protocol = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let host = without_protocol
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");
    is_valid_local_host(host)
}

fn is_private_ip(host: &str) -> bool {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    let nums: Option<Vec<u8>> = parts.iter().map(|part| part.parse::<u8>().ok()).collect();
    if let Some([a, b, _, _]) = nums.as_deref() {
        if *a == 10 {
            return true;
        }
        if *a == 172 && (16..=31).contains(b) {
            return true;
        }
        if *a == 192 && *b == 168 {
            return true;
        }
        if *a == 169 && *b == 254 {
            return true;
        }
    }
    false
}

fn strip_ansi(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
                    break;
                }
            }
            continue;
        }
        output.push(ch);
    }
    output
}
