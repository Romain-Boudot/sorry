use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::auth::AuthUser;
use crate::state::AppState;

fn is_public_ip(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(ip) => {
            let o = ip.octets();
            !(ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_multicast()
                || ip.is_unspecified()
                || ip.is_documentation()
                // CGNAT 100.64.0.0/10
                || (o[0] == 100 && (o[1] & 0xc0) == 64)
                // Benchmarking 198.18.0.0/15
                || (o[0] == 198 && (o[1] & 0xfe) == 18))
        }
        IpAddr::V6(ip) => {
            if let Some(v4) = ip.to_ipv4_mapped() {
                return is_public_ip(IpAddr::V4(v4));
            }
            let s = ip.segments();
            !(ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_multicast()
                // fe80::/10 link-local
                || (s[0] & 0xffc0) == 0xfe80
                // fc00::/7 unique local
                || (s[0] & 0xfe00) == 0xfc00)
        }
    }
}

struct PublicOnlyResolver;

impl reqwest::dns::Resolve for PublicOnlyResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_string();
        Box::pin(async move {
            let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0))
                .await
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?
                .collect();
            if addrs.is_empty() {
                return Err::<reqwest::dns::Addrs, _>("no addresses returned".into());
            }
            if !addrs.iter().all(|sa| is_public_ip(sa.ip())) {
                return Err::<reqwest::dns::Addrs, _>("host resolves to non-public address".into());
            }
            let iter: reqwest::dns::Addrs = Box::new(addrs.into_iter());
            Ok(iter)
        })
    }
}

fn safe_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::limited(5))
        .dns_resolver(Arc::new(PublicOnlyResolver))
}

#[derive(Deserialize)]
pub struct OgRequest {
    url: String,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct OgData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_name: Option<String>,
    pub url: String,
}

pub async fn fetch_og(
    _auth: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<OgRequest>,
) -> Result<Json<OgData>, StatusCode> {
    // Validate URL
    if !payload.url.starts_with("http://") && !payload.url.starts_with("https://") {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check cache
    {
        let cache = state.og_cache.read().unwrap();
        if let Some((data, ts)) = cache.get(&payload.url) {
            if ts.elapsed() < Duration::from_secs(3600) {
                return Ok(Json(data.clone()));
            }
        }
    }

    // Try oEmbed first for known providers
    if let Some(oembed_url) = get_oembed_url(&payload.url) {
        if let Ok(og) = fetch_oembed(&oembed_url, &payload.url).await {
            // Cache and return
            let mut cache = state.og_cache.write().unwrap();
            cache.insert(payload.url.clone(), (og.clone(), Instant::now()));
            return Ok(Json(og));
        }
    }

    // Fallback: fetch OG tags from HTML
    let response = safe_client_builder()
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .get(&payload.url)
        .header("User-Agent", "Mozilla/5.0 (compatible; SorryBot/1.0; +https://github.com/Romain-Boudot/sorry)")
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    // Only parse HTML responses
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !content_type.contains("text/html") {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    // Limit body size to 512KB
    let body = response
        .text()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let limited = if body.len() > 512 * 1024 {
        &body[..512 * 1024]
    } else {
        body.as_str()
    };

    let mut og = parse_og_tags(limited, &payload.url);

    // Try to discover and fetch oEmbed from <link> tag
    if let Some(oembed_link) = find_oembed_link(limited) {
        if let Ok(oembed_data) = fetch_oembed(&oembed_link, &payload.url).await {
            // oEmbed enriches: fill in missing fields
            if og.title.is_none() { og.title = oembed_data.title; }
            if og.description.is_none() { og.description = oembed_data.description; }
            if og.image.is_none() { og.image = oembed_data.image; }
            if og.site_name.is_none() { og.site_name = oembed_data.site_name; }
        }
    }

    // Cache result
    {
        let mut cache = state.og_cache.write().unwrap();
        if cache.len() > 1000 {
            let expired: Vec<String> = cache
                .iter()
                .filter(|(_, (_, ts))| ts.elapsed() > Duration::from_secs(3600))
                .map(|(k, _)| k.clone())
                .collect();
            for k in expired {
                cache.remove(&k);
            }
        }
        cache.insert(payload.url.clone(), (og.clone(), Instant::now()));
    }

    Ok(Json(og))
}

fn parse_og_tags(html: &str, url: &str) -> OgData {
    let mut data = OgData {
        url: url.to_string(),
        ..Default::default()
    };
    let mut fallback_description: Option<String> = None;
    let mut has_og_description = false;

    // Find all <meta ... > tags and extract OG properties
    let lower = html.to_lowercase();
    let mut pos = 0;
    while let Some(start) = lower[pos..].find("<meta") {
        let abs_start = pos + start;
        let Some(end) = lower[abs_start..].find('>') else { break };
        let tag = &html[abs_start..abs_start + end + 1];
        pos = abs_start + end + 1;

        let property = extract_attr(tag, "property").or_else(|| extract_attr(tag, "name"));
        let content = extract_attr(tag, "content");

        if let (Some(prop), Some(cont)) = (property, content) {
            match prop.to_lowercase().as_str() {
                "og:title" => data.title = Some(truncate(&cont, 200)),
                "og:description" => {
                    data.description = Some(truncate(&cont, 500));
                    has_og_description = true;
                }
                "description" => {
                    if !has_og_description {
                        fallback_description = Some(truncate(&cont, 500));
                    }
                }
                "og:image" => data.image = Some(cont),
                "og:site_name" => data.site_name = Some(truncate(&cont, 100)),
                _ => {}
            }
        }
    }

    // Fallback description
    if data.description.is_none() {
        data.description = fallback_description;
    }

    // Fallback: <title> tag if no og:title
    if data.title.is_none() {
        if let Some(start) = lower.find("<title>") {
            if let Some(end) = lower[start + 7..].find("</title>") {
                let title = &html[start + 7..start + 7 + end];
                data.title = Some(truncate(title.trim(), 200));
            }
        }
    }

    data
}

fn extract_attr(tag: &str, name: &str) -> Option<String> {
    let tag_lower = tag.to_lowercase();
    for quote in ['"', '\''] {
        let pattern = format!("{}={}", name, quote);
        if let Some(start) = tag_lower.find(&pattern) {
            let value_start = start + pattern.len();
            if let Some(end) = tag[value_start..].find(quote) {
                return Some(html_decode(&tag[value_start..value_start + end]));
            }
        }
    }
    None
}

fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

fn find_oembed_link(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let mut pos = 0;
    while let Some(start) = lower[pos..].find("<link") {
        let abs_start = pos + start;
        let Some(end) = lower[abs_start..].find('>') else { break };
        let tag = &html[abs_start..abs_start + end + 1];
        pos = abs_start + end + 1;

        let rel = extract_attr(tag, "rel").unwrap_or_default();
        let typ = extract_attr(tag, "type").unwrap_or_default();
        if rel == "alternate" && (typ.contains("oembed") || typ.contains("json+oembed")) {
            if let Some(href) = extract_attr(tag, "href") {
                return Some(href);
            }
        }
    }
    None
}

// ── oEmbed ──

struct OembedProvider {
    patterns: &'static [&'static str],
    endpoint: &'static str,
}

const OEMBED_PROVIDERS: &[OembedProvider] = &[
    OembedProvider {
        patterns: &[
            "youtube.com/watch",
            "youtu.be/",
            "youtube.com/shorts/",
        ],
        endpoint: "https://www.youtube.com/oembed",
    },
    OembedProvider {
        patterns: &["twitter.com/", "x.com/"],
        endpoint: "https://publish.twitter.com/oembed",
    },
    OembedProvider {
        patterns: &["open.spotify.com/"],
        endpoint: "https://open.spotify.com/oembed",
    },
    OembedProvider {
        patterns: &["vimeo.com/"],
        endpoint: "https://vimeo.com/api/oembed.json",
    },
];

fn get_oembed_url(url: &str) -> Option<String> {
    for provider in OEMBED_PROVIDERS {
        for pattern in provider.patterns {
            if url.contains(pattern) {
                return Some(format!(
                    "{}?url={}&format=json",
                    provider.endpoint,
                    urlencoding::encode(url)
                ));
            }
        }
    }
    None
}

#[derive(Deserialize)]
struct OembedResponse {
    title: Option<String>,
    author_name: Option<String>,
    provider_name: Option<String>,
    thumbnail_url: Option<String>,
}

async fn fetch_oembed(oembed_url: &str, original_url: &str) -> Result<OgData, ()> {
    let response = safe_client_builder()
        .build()
        .map_err(|_| ())?
        .get(oembed_url)
        .send()
        .await
        .map_err(|_| ())?;

    if !response.status().is_success() {
        return Err(());
    }

    let data: OembedResponse = response.json().await.map_err(|_| ())?;

    Ok(OgData {
        title: data.title,
        description: data.author_name.map(|a| format!("par {}", a)),
        image: data.thumbnail_url,
        site_name: data.provider_name,
        url: original_url.to_string(),
    })
}
