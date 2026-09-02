use std::net::IpAddr;
use std::process::Command;

use super::process::silent_command;

pub(super) fn apply_git_proxy_env(command: &mut Command, repo_path: &str) {
    if should_bypass_proxy_for_repo(repo_path) {
        clear_proxy_env(command);
        return;
    }

    let proxy = resolve_git_proxy_env(repo_path);
    set_proxy_env_pair(command, "HTTP_PROXY", "http_proxy", proxy.http.as_deref());
    set_proxy_env_pair(
        command,
        "HTTPS_PROXY",
        "https_proxy",
        proxy.https.as_deref(),
    );
    set_proxy_env_pair(command, "ALL_PROXY", "all_proxy", proxy.all.as_deref());
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct GitProxyEnv {
    pub(super) http: Option<String>,
    pub(super) https: Option<String>,
    pub(super) all: Option<String>,
}

pub(super) fn remote_host_from_url(remote_url: &str) -> Option<String> {
    let remote_url = remote_url.trim();
    if remote_url.is_empty() {
        return None;
    }

    if let Some((_, rest)) = remote_url.split_once("://") {
        let authority = rest.split(['/', '?', '#']).next()?.trim();
        let host_port = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
        return Some(strip_port(host_port)?.to_ascii_lowercase());
    }

    if let Some((user_host, _path)) = remote_url.split_once(':') {
        if let Some((_, host)) = user_host.rsplit_once('@') {
            return Some(host.trim().to_ascii_lowercase());
        }
    }

    None
}

pub(super) fn is_proxy_bypass_host(host: &str) -> bool {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() {
        return false;
    }
    if host == "localhost" || host.ends_with(".local") || !host.contains('.') {
        return true;
    }

    if let Ok(IpAddr::V4(ip)) = host.parse::<IpAddr>() {
        let octets = ip.octets();
        return octets[0] == 10
            || (octets[0] == 172 && (16..=31).contains(&octets[1]))
            || (octets[0] == 192 && octets[1] == 168)
            || octets[0] == 127
            || (octets[0] == 169 && octets[1] == 254);
    }

    false
}

pub(super) fn no_proxy_rule_matches_host(rule: &str, host: &str) -> bool {
    if rule == "*" {
        return true;
    }
    if rule == "<local>" {
        return !host.contains('.');
    }
    if let Some(prefix) = rule.strip_suffix('*') {
        return host.starts_with(prefix);
    }
    if let Some(suffix) = rule.strip_prefix("*.") {
        return host == suffix || host.ends_with(&format!(".{}", suffix));
    }
    if let Some(suffix) = rule.strip_prefix('.') {
        return host == suffix || host.ends_with(&format!(".{}", suffix));
    }

    host == rule
}

pub(super) fn registry_value(text: &str, name: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let line = line.trim();
        if !line.starts_with(name) {
            return None;
        }

        let mut parts = line.split_whitespace();
        let value_name = parts.next()?;
        if value_name != name {
            return None;
        }

        let _value_type = parts.next()?;
        let value = parts.collect::<Vec<_>>().join(" ");
        if value.trim().is_empty() {
            None
        } else {
            Some(value)
        }
    })
}

pub(super) fn proxy_env_from_windows_proxy_server(value: &str) -> Option<GitProxyEnv> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if !value.contains('=') {
        let proxy = normalize_http_proxy_value(value)?;
        return Some(GitProxyEnv {
            http: Some(proxy.clone()),
            https: Some(proxy.clone()),
            all: Some(proxy),
        });
    }

    let mut proxy = GitProxyEnv::default();
    for part in value.split(';') {
        let Some((kind, address)) = part.split_once('=') else {
            continue;
        };
        let kind = kind.trim().to_ascii_lowercase();
        match kind.as_str() {
            "http" => proxy.http = normalize_http_proxy_value(address),
            "https" => proxy.https = normalize_http_proxy_value(address),
            "socks" => proxy.all = normalize_socks_proxy_value(address),
            _ => {}
        }
    }

    if proxy.https.is_none() {
        proxy.https = proxy.http.clone().or_else(|| proxy.all.clone());
    }
    if proxy.http.is_none() {
        proxy.http = proxy.https.clone().or_else(|| proxy.all.clone());
    }
    if proxy.all.is_none() {
        proxy.all = proxy.https.clone().or_else(|| proxy.http.clone());
    }

    if proxy.http.is_none() && proxy.https.is_none() && proxy.all.is_none() {
        None
    } else {
        Some(proxy)
    }
}

fn clear_proxy_env(command: &mut Command) {
    for key in [
        "HTTP_PROXY",
        "http_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        command.env_remove(key);
    }
}

fn set_proxy_env_pair(command: &mut Command, upper: &str, lower: &str, value: Option<&str>) {
    if let Some(value) = value {
        command.env(upper, value);
        command.env(lower, value);
    }
}

fn resolve_git_proxy_env(_repo_path: &str) -> GitProxyEnv {
    let mut proxy = GitProxyEnv {
        http: env_proxy(&["HTTP_PROXY", "http_proxy"]),
        https: env_proxy(&["HTTPS_PROXY", "https_proxy"]),
        all: env_proxy(&["ALL_PROXY", "all_proxy"]),
    };

    if proxy.http.is_none() && proxy.https.is_none() && proxy.all.is_none() {
        if let Some(system_proxy) = windows_system_proxy() {
            proxy = system_proxy;
        }
    }

    if proxy.https.is_none() {
        proxy.https = proxy.http.clone().or_else(|| proxy.all.clone());
    }
    if proxy.http.is_none() {
        proxy.http = proxy.https.clone().or_else(|| proxy.all.clone());
    }
    if proxy.all.is_none() {
        proxy.all = proxy.https.clone().or_else(|| proxy.http.clone());
    }

    proxy
}

fn should_bypass_proxy_for_repo(repo_path: &str) -> bool {
    let Some(remote_url) = remote_origin_url(repo_path) else {
        return false;
    };
    let Some(host) = remote_host_from_url(&remote_url) else {
        return false;
    };

    is_proxy_bypass_host(&host) || host_matches_no_proxy(&host)
}

fn remote_origin_url(repo_path: &str) -> Option<String> {
    let output = silent_command("git")
        .args([
            "-c",
            "core.quotePath=false",
            "config",
            "--get",
            "remote.origin.url",
        ])
        .current_dir(repo_path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn strip_port(host_port: &str) -> Option<&str> {
    let host_port = host_port.trim();
    if host_port.is_empty() {
        return None;
    }
    if let Some(rest) = host_port.strip_prefix('[') {
        let (host, _) = rest.split_once(']')?;
        return Some(host);
    }

    Some(host_port.split(':').next().unwrap_or(host_port))
}

fn host_matches_no_proxy(host: &str) -> bool {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() {
        return false;
    }

    let no_proxy = [
        std::env::var("NO_PROXY").ok(),
        std::env::var("no_proxy").ok(),
        windows_proxy_override(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(",");

    no_proxy
        .split([',', ';', ' '])
        .filter_map(|item| {
            let item = item.trim().trim_end_matches('.').to_ascii_lowercase();
            if item.is_empty() {
                None
            } else {
                Some(item)
            }
        })
        .any(|rule| no_proxy_rule_matches_host(&rule, &host))
}

fn env_proxy(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        std::env::var(key)
            .ok()
            .and_then(|value| normalize_http_proxy_value(&value))
    })
}

fn normalize_http_proxy_value(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if value.contains("://") {
        return Some(value.to_string());
    }

    Some(format!("http://{}", value))
}

fn normalize_socks_proxy_value(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if value.contains("://") {
        return Some(value.to_string());
    }

    Some(format!("socks5://{}", value))
}

#[cfg(windows)]
fn windows_system_proxy() -> Option<GitProxyEnv> {
    let output = silent_command("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let enabled = registry_value(&text, "ProxyEnable")
        .is_some_and(|value| value == "0x1" || value == "1");
    if !enabled {
        return None;
    }

    registry_value(&text, "ProxyServer")
        .and_then(|value| proxy_env_from_windows_proxy_server(&value))
}

#[cfg(windows)]
fn windows_proxy_override() -> Option<String> {
    let output = silent_command("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    registry_value(&text, "ProxyOverride")
}

#[cfg(not(windows))]
fn windows_proxy_override() -> Option<String> {
    None
}

#[cfg(not(windows))]
fn windows_system_proxy() -> Option<GitProxyEnv> {
    None
}
