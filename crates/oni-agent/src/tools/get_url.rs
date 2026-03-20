use super::Tool;
use oni_core::error::Result;
use oni_core::types::ToolCapability;

pub struct GetUrlTool;

const MAX_BYTES: usize = 50 * 1024; // 50 KB

fn strip_html_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    // Collapse runs of whitespace/newlines for readability
    let mut result = String::with_capacity(out.len());
    let mut prev_whitespace = false;
    for ch in out.chars() {
        if ch.is_whitespace() {
            if !prev_whitespace {
                result.push('\n');
            }
            prev_whitespace = true;
        } else {
            result.push(ch);
            prev_whitespace = false;
        }
    }
    result
}

impl Tool for GetUrlTool {
    fn name(&self) -> &str {
        "get_url"
    }

    fn required_capabilities(&self) -> &[ToolCapability] {
        &[ToolCapability::NetworkFetch]
    }

    fn description(&self) -> &str {
        "Fetch content from a URL. Returns the text content (HTML stripped to text for web pages)."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "get_url",
                "description": self.description(),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "URL to fetch"
                        }
                    },
                    "required": ["url"]
                }
            }
        })
    }

    fn execute(&self, args: serde_json::Value) -> Result<String> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| oni_core::error::err!("Missing 'url' argument"))?
            .to_string();

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Ok(format!("Error: only http:// and https:// URLs are allowed, got: {}", url));
        }
        let is_private = url.contains("://localhost") || url.contains("://127.0.0.1")
            || url.contains("://0.0.0.0") || url.contains("://169.254.")
            || url.contains("://10.") || url.contains("://192.168.");
        if is_private {
            return Ok("Error: fetching private/internal network addresses is not allowed.".into());
        }

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let client = reqwest::Client::builder()
                    .user_agent("oni-agent/0.1")
                    .build()?;

                let response = client.get(&url).send().await?;
                let content_type = response
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();

                let bytes = response.bytes().await?;
                let truncated = bytes.len() > MAX_BYTES;
                let slice = if truncated { &bytes[..MAX_BYTES] } else { &bytes[..] };
                let text = String::from_utf8_lossy(slice).into_owned();

                let is_html = content_type.contains("text/html");

                let body = if is_html {
                    strip_html_tags(&text)
                } else {
                    text
                };

                let mut out = body;
                if truncated {
                    out.push_str(&format!(
                        "\n\n[Truncated: showing first 50KB of {} bytes]",
                        bytes.len()
                    ));
                }

                Ok::<String, reqwest::Error>(out)
            })
        });

        match result {
            Ok(content) => Ok(content),
            Err(e) => Ok(format!("Error fetching URL: {}", e)),
        }
    }
}
