//! Web tools: web_search and web_fetch

use async_trait::async_trait;
use regex::Regex;
use serde::Deserialize;
use serde_json::json;
use tracing::debug;

use super::ToolTrait;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36";

/// Web search tool using Brave Search API
pub struct WebSearchTool {
    api_key: String,
    max_results: u32,
}

impl WebSearchTool {
    /// Create new WebSearchTool with API key and max_results from config
    pub fn new(api_key: Option<String>, max_results: u32) -> Self {
        let api_key = api_key
            .or_else(|| std::env::var("BRAVE_API_KEY").ok())
            .unwrap_or_default();
        Self {
            api_key,
            max_results,
        }
    }

    /// Create from config
    pub fn from_config(config: &opensam_config::Config) -> Self {
        let api_key = config.brave_api_key();
        let max_results = config.web_search_max_results();
        Self::new(api_key, max_results)
    }
}

#[derive(Deserialize)]
struct WebSearchArgs {
    query: String,
    count: Option<u32>,
}

#[async_trait]
impl ToolTrait for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }
    fn description(&self) -> &str {
        "Search the web. Returns titles, URLs, and snippets."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query" },
                "count": { "type": "integer", "description": "Number of results (1-10)", "minimum": 1, "maximum": 10 }
            },
            "required": ["query"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if self.api_key.is_empty() {
            return Ok("Error: BRAVE_API_KEY not configured".to_string());
        }
        let args: WebSearchArgs = serde_json::from_value(args)?;
        let count = args.count.unwrap_or(self.max_results).clamp(1, 10);
        debug!("Web search: {}", args.query);

        let client = reqwest::Client::new();
        let response = client
            .get("https://api.search.brave.com/res/v1/web/search")
            .query(&[("q", &args.query), ("count", &count.to_string())])
            .header("Accept", "application/json")
            .header("X-Subscription-Token", &self.api_key)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Ok(format!("Error: Search API returned {}", status));
        }

        let data: serde_json::Value = response.json().await?;
        let results = data
            .get("web")
            .and_then(|w| w.get("results"))
            .and_then(|r| r.as_array());

        if results.is_none() || results.unwrap().is_empty() {
            return Ok(format!("No results for: {}", args.query));
        }

        let mut lines = vec![format!("Results for: {}", args.query)];
        for (i, item) in results.unwrap().iter().take(count as usize).enumerate() {
            let title = item.get("title").and_then(|t| t.as_str()).unwrap_or("");
            let url = item.get("url").and_then(|u| u.as_str()).unwrap_or("");
            let desc = item
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");
            lines.push(format!("{}. {}", i + 1, title));
            lines.push(format!("   {}", url));
            if !desc.is_empty() {
                lines.push(format!("   {}", desc));
            }
        }
        Ok(lines.join("\n"))
    }
}

pub struct WebFetchTool {
    max_chars: usize,
}
impl WebFetchTool {
    pub fn new(max_chars: usize) -> Self {
        Self { max_chars }
    }
}
impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new(50000)
    }
}

#[derive(Deserialize)]
struct WebFetchArgs {
    url: String,
    #[serde(rename = "extractMode")]
    extract_mode: Option<String>,
    #[serde(rename = "maxChars")]
    max_chars: Option<usize>,
}

#[async_trait]
impl ToolTrait for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }
    fn description(&self) -> &str {
        "Fetch URL and extract readable content."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "URL to fetch" },
                "extractMode": { "type": "string", "enum": ["markdown", "text"], "default": "markdown" },
                "maxChars": { "type": "integer", "minimum": 100 }
            },
            "required": ["url"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: WebFetchArgs = serde_json::from_value(args)?;
        let max_chars = args.max_chars.unwrap_or(self.max_chars);
        let extract_mode = args.extract_mode.as_deref().unwrap_or("markdown");
        debug!("Fetching URL: {} (mode: {})", args.url, extract_mode);

        let client = reqwest::Client::new();
        let response = client
            .get(&args.url)
            .header("User-Agent", USER_AGENT)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;

        let status = response.status();
        let final_url = response.url().clone();
        let headers = response.headers().clone();
        let content_type = headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let text = response.text().await?;

        let (content, extractor) = if content_type.contains("application/json") {
            (text, "json")
        } else if extract_mode == "text" {
            (strip_tags(&text), "text")
        } else {
            (html_to_markdown(&text), "markdown")
        };

        let truncated = content.len() > max_chars;
        let content = if truncated {
            content[..max_chars].to_string()
        } else {
            content
        };

        Ok(json!({
            "url": args.url, "finalUrl": final_url.as_str(), "status": status.as_u16(),
            "extractor": extractor, "truncated": truncated, "length": content.len(), "text": content
        })
        .to_string())
    }
}

fn strip_tags(html: &str) -> String {
    let re = Regex::new(r"(?is)<script[\s\S]*?</script>|<style[\s\S]*?</style>").unwrap();
    let text = re.replace_all(html, "");
    let re = Regex::new(r"<[^>]+>").unwrap();
    let text = re.replace_all(&text, "");
    decode_html_entities(&text).trim().to_string()
}

fn html_to_markdown(html: &str) -> String {
    // Remove script and style tags with content
    let re = Regex::new(r"(?is)<script[\s\S]*?</script>|<style[\s\S]*?</style>|<nav[\s\S]*?</nav>|<header[\s\S]*?</header>|<footer[\s\S]*?</footer>").unwrap();
    let html = re.replace_all(html, "");

    let mut markdown = String::new();
    let mut in_code_block = false;
    let mut list_stack: Vec<&str> = Vec::new();

    // Pre-compile regex patterns
    let tag_re = Regex::new(r"(?is)<(/?)([a-z0-9]+)[^>]*?>|([^<]+)").unwrap();
    let whitespace_re = Regex::new(r"\s+").unwrap();
    let href_re = Regex::new(r#"href=["']([^"']+)["']"#).unwrap();

    for cap in tag_re.captures_iter(&html) {
        let closing = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let tag = cap
            .get(2)
            .map(|m| m.as_str().to_lowercase())
            .unwrap_or_default();
        let text = cap.get(3).map(|m| m.as_str()).unwrap_or("");

        if !text.is_empty() {
            let decoded = decode_html_entities(text);
            let trimmed = decoded.trim();
            if !trimmed.is_empty() {
                if in_code_block {
                    markdown.push_str(&decoded);
                } else {
                    // Collapse whitespace for regular text
                    let collapsed = whitespace_re.replace_all(trimmed, " ");
                    if !markdown.is_empty() && !markdown.ends_with(['\n', ' ']) {
                        markdown.push(' ');
                    }
                    markdown.push_str(&collapsed);
                }
            }
            continue;
        }

        let is_closing = closing == "/";

        match tag.as_str() {
            "h1" => {
                if is_closing {
                    markdown.push_str("\n\n");
                } else {
                    markdown.push_str("\n\n# ");
                }
            }
            "h2" => {
                if is_closing {
                    markdown.push_str("\n\n");
                } else {
                    markdown.push_str("\n\n## ");
                }
            }
            "h3" => {
                if is_closing {
                    markdown.push_str("\n\n");
                } else {
                    markdown.push_str("\n\n### ");
                }
            }
            "h4" => {
                if is_closing {
                    markdown.push_str("\n\n");
                } else {
                    markdown.push_str("\n\n#### ");
                }
            }
            "h5" => {
                if is_closing {
                    markdown.push_str("\n\n");
                } else {
                    markdown.push_str("\n\n##### ");
                }
            }
            "h6" => {
                if is_closing {
                    markdown.push_str("\n\n");
                } else {
                    markdown.push_str("\n\n###### ");
                }
            }
            "p" | "div" | "section" | "article" | "main" | "aside" => {
                if is_closing {
                    markdown.push_str("\n\n");
                }
            }
            "br" => {
                markdown.push('\n');
            }
            "hr" => {
                markdown.push_str("\n\n---\n\n");
            }
            "strong" | "b" => {
                markdown.push_str("**");
            }
            "em" | "i" => {
                markdown.push('*');
            }
            "code" => {
                if is_closing {
                    markdown.push('`');
                    in_code_block = false;
                } else {
                    markdown.push('`');
                    in_code_block = true;
                }
            }
            "pre" => {
                markdown.push_str("\n```\n");
            }
            "a" => {
                if is_closing {
                    // Extract href from the opening tag - we need a different approach
                    // For now, just output the link text
                } else {
                    // Look ahead for href
                    if let Some(href_cap) = href_re.captures(&cap[0]) {
                        let _href = &href_cap[1];
                        markdown.push('[');
                        // We'll need to find the text and closing tag
                        // Simplified: just output link marker
                    }
                }
            }
            "ul" => {
                if is_closing {
                    list_stack.pop();
                } else {
                    list_stack.push("ul");
                }
                markdown.push('\n');
            }
            "ol" => {
                if is_closing {
                    list_stack.pop();
                } else {
                    list_stack.push("ol");
                }
                markdown.push('\n');
            }
            "li" => {
                if is_closing {
                    markdown.push('\n');
                } else {
                    let indent = "  ".repeat(list_stack.len().saturating_sub(1));
                    if list_stack.last() == Some(&"ol") {
                        markdown.push_str(&format!("{}1. ", indent));
                    } else {
                        markdown.push_str(&format!("{}- ", indent));
                    }
                }
            }
            "blockquote" => {
                if is_closing {
                    markdown.push('\n');
                } else {
                    markdown.push_str("> ");
                }
            }
            _ => {} // Ignore other tags
        }
    }

    // Post-processing for links - simpler approach
    let link_re = Regex::new(r#"<a[^>]+href=["']([^"']+)["'][^>]*>([^<]*)</a>"#).unwrap();
    let result = link_re.replace_all(&markdown, |caps: &regex::Captures| {
        let href = &caps[1];
        let text = &caps[2];
        if text.is_empty() {
            format!("<{ }>", href)
        } else {
            format!("[{}]({})", text, href)
        }
    });

    // Clean up excessive newlines
    let result = Regex::new(r"\n{3,}").unwrap().replace_all(&result, "\n\n");

    result.trim().to_string()
}

fn decode_html_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&ndash;", "–")
        .replace("&mdash;", "—")
        .replace("&hellip;", "…")
}
