use std::collections::VecDeque;

use super::ProjectProcessLogLine;

pub(super) const MAX_DETECTED_PORTS: usize = 4;

#[allow(dead_code)]
pub(super) fn detect_ports(lines: &VecDeque<ProjectProcessLogLine>) -> Vec<u16> {
    let mut ports = Vec::new();
    for line in lines.iter().rev() {
        append_detected_ports(&line.text, &mut ports);
        if ports.len() >= MAX_DETECTED_PORTS {
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

fn is_access_or_request_log(lower: &str) -> bool {
    let has_http_method = [
        "\"get ", "\"post ", "\"put ", "\"delete ", "\"patch ", "\"head ", "\"options ",
        "\"connect ", "\"trace ", "\"ws ", "\"websocket ",
        " get /", " post /", " put /", " delete /", " patch /", " head /", " options /",
        "get http://", "post http://", "get https://", "post https://",
    ]
    .iter()
    .any(|m| lower.contains(m));

    if has_http_method {
        return true;
    }

    if lower.contains(" - \"") || lower.contains(" - - [") {
        return true;
    }

    if (lower.contains("http/1.0")
        || lower.contains("http/1.1")
        || lower.contains("http/2")
        || lower.contains("http/3"))
        && (lower.contains("\" 2")
            || lower.contains("\" 3")
            || lower.contains("\" 4")
            || lower.contains("\" 5")
            || lower.contains(" 200 ")
            || lower.contains(" 201 ")
            || lower.contains(" 204 ")
            || lower.contains(" 304 ")
            || lower.contains(" 404 ")
            || lower.contains(" 500 "))
    {
        return true;
    }

    lower.contains("connection from")
        || lower.contains("connected from")
        || lower.contains("accepted connection")
        || lower.contains("closed connection")
        || lower.contains("client disconnected")
        || lower.contains("remote client")
        || lower.contains("peer disconnected")
}

fn is_outbound_or_client_log(lower: &str) -> bool {
    lower.contains("connecting")
        || lower.contains("connected to")
        || lower.contains("connection to")
        || lower.contains("connect to")
        || lower.contains("proxying")
        || lower.contains("proxied")
        || (lower.contains("proxy") && (lower.contains("to ") || lower.contains("target")))
        || lower.contains("forwarding")
        || lower.contains("forwarded")
        || (lower.contains("forward") && lower.contains("to "))
        || lower.contains("dispatching")
        || lower.contains("upstream")
        || lower.contains("request to")
        || lower.contains("response from")
        || lower.contains("sending to")
        || lower.contains("fetching")
        || lower.contains("redis://")
        || lower.contains("postgres://")
        || lower.contains("postgresql://")
        || lower.contains("mysql://")
        || lower.contains("mongodb://")
        || lower.contains("amqp://")
        || lower.contains("client:")
        || lower.contains("remote:")
}

fn has_server_listening_context(lower: &str) -> bool {
    lower.contains("listen")
        || lower.contains("server")
        || lower.contains("serve")
        || lower.contains("serving")
        || lower.contains("bound")
        || lower.contains("bind")
        || lower.contains("started")
        || lower.contains("running")
        || lower.contains("ready")
        || lower.contains("local:")
        || lower.contains("network:")
        || lower.contains("address:")
        || lower.contains("host:")
        || lower.contains("url:")
        || lower.contains("app at")
        || lower.contains("web at")
        || lower.contains("端口")
}

fn is_ephemeral_port(port: u16) -> bool {
    port >= 49152
}

pub(super) fn append_detected_ports(text: &str, ports: &mut Vec<u16>) {
    if ports.len() >= MAX_DETECTED_PORTS {
        return;
    }

    let text = strip_ansi(text);
    let lower = text.to_lowercase();
    if !lower.contains(':') && !lower.contains("port") && !text.contains("端口") {
        return;
    }

    // Skip access/request logs and outbound client connections
    if is_access_or_request_log(&lower) || is_outbound_or_client_log(&lower) {
        return;
    }

    let has_listening_context = has_server_listening_context(&lower);

    // 1. Check for localhost URLs and host:port tokens
    for token in text.split_whitespace() {
        if ports.len() >= MAX_DETECTED_PORTS {
            break;
        }

        let is_explicit_url = token.starts_with("http://") || token.starts_with("https://");

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
            // Naked host:port tokens without protocol require explicit server listening context,
            // and unprompted ephemeral client ports (>= 49152) are rejected.
            if !is_explicit_url && (!has_listening_context || is_ephemeral_port(port)) {
                continue;
            }
            if !ports.contains(&port) {
                ports.push(port);
            }
        }
    }

    // 2. Check for explicit "port" / "端口" keywords: e.g. "port: 3000", "port 3000", "port:3000", "PORT 8080", "端口: 8000", "监听端口: 8080"
    for keyword in ["port", "端口"] {
        let mut search_from = 0;
        while let Some(pos) = lower[search_from..].find(keyword) {
            if ports.len() >= MAX_DETECTED_PORTS {
                break;
            }
            let actual_pos = search_from + pos;
            let after_keyword = &lower[actual_pos + keyword.len()..];
            let rest = after_keyword.trim_start_matches(|c: char| c == ':' || c == '=' || c == '：' || c.is_whitespace());
            let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
            if let Ok(port) = digits.parse::<u16>() {
                if (1024..=65535).contains(&port) && !ports.contains(&port) {
                    ports.push(port);
                }
            }
            search_from = actual_pos + keyword.len();
        }
    }
}

pub(super) fn append_detected_urls(text: &str, urls: &mut Vec<String>) {
    if !text.contains("http://") && !text.contains("https://") {
        return;
    }
    let text = strip_ansi(text);
    let lower = text.to_lowercase();
    if is_access_or_request_log(&lower) || is_outbound_or_client_log(&lower) {
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
