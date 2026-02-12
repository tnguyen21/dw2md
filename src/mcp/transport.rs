use anyhow::{Context, Result, bail};
use reqwest::Client;

use super::types::JsonRpcRequest;

/// Parsed response from the MCP server — handles both JSON and SSE responses.
pub struct McpResponse {
    pub body: serde_json::Value,
    pub session_id: Option<String>,
}

/// Send a JSON-RPC request to the MCP endpoint and parse the response.
///
/// Handles both `application/json` and `text/event-stream` (SSE) content types.
pub async fn send_request(
    client: &Client,
    endpoint: &str,
    request: &JsonRpcRequest,
    session_id: Option<&str>,
    timeout: std::time::Duration,
) -> Result<McpResponse> {
    let mut builder = client
        .post(endpoint)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .timeout(timeout);

    if let Some(sid) = session_id {
        builder = builder.header("Mcp-Session-Id", sid);
    }

    let response = builder
        .json(request)
        .send()
        .await
        .context("Failed to send request to MCP server")?;

    let new_session_id = response
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!(
            "MCP server returned HTTP {}: {}",
            status.as_u16(),
            body.chars().take(500).collect::<String>()
        );
    }

    if content_type.contains("text/event-stream") {
        let text = response.text().await.context("Failed to read SSE body")?;
        let body = parse_sse(&text)?;
        Ok(McpResponse {
            body,
            session_id: new_session_id,
        })
    } else {
        let body: serde_json::Value = response.json().await.context("Failed to parse JSON response")?;
        Ok(McpResponse {
            body,
            session_id: new_session_id,
        })
    }
}

/// Parse an SSE stream body, extracting the last JSON-RPC message from `data:` lines.
///
/// The MCP server sends SSE events where each `data:` line contains a JSON-RPC message.
/// We want the final complete message (which contains the result).
fn parse_sse(text: &str) -> Result<serde_json::Value> {
    let mut last_message: Option<serde_json::Value> = None;

    for line in text.lines() {
        let line = line.trim();
        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim();
            if data.is_empty() {
                continue;
            }
            match serde_json::from_str::<serde_json::Value>(data) {
                Ok(value) => {
                    last_message = Some(value);
                }
                Err(_) => continue,
            }
        }
    }

    last_message.context("No valid JSON-RPC message found in SSE stream")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_single_message() {
        let input = r#"event: message
data: {"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"hello"}]}}

"#;
        let result = parse_sse(input).unwrap();
        assert_eq!(result["id"], 1);
        assert_eq!(result["result"]["content"][0]["text"], "hello");
    }

    #[test]
    fn test_parse_sse_multiple_messages() {
        let input = r#"data: {"jsonrpc":"2.0","method":"progress"}

data: {"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"final"}]}}

"#;
        let result = parse_sse(input).unwrap();
        assert_eq!(result["result"]["content"][0]["text"], "final");
    }

    #[test]
    fn test_parse_sse_empty() {
        let result = parse_sse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_sse_no_data_lines() {
        let result = parse_sse("event: ping\n\n");
        assert!(result.is_err());
    }
}
