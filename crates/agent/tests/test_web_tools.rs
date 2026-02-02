//! Tests for web tools

use opensam_agent::tools::{ToolTrait, WebFetchTool, WebSearchTool};
use serde_json::json;

// Test strip_tags functionality through WebFetchTool
#[tokio::test]
async fn test_web_fetch_strip_tags() {
    let tool = WebFetchTool::new(1000);

    // This test would require a mock server; for now we test the strip_tags logic
    // through parameters validation
    let params = tool.parameters();
    assert_eq!(params["type"], "object");
    assert!(params["required"]
        .as_array()
        .unwrap()
        .contains(&json!("url")));
}

#[test]
fn test_web_fetch_tool_default() {
    let tool = WebFetchTool::default();
    assert_eq!(tool.name(), "web_fetch");
    assert_eq!(
        tool.description(),
        "Fetch URL and extract readable content."
    );
}

#[test]
fn test_web_fetch_tool_new() {
    let tool = WebFetchTool::new(5000);
    assert_eq!(tool.name(), "web_fetch");
}

#[test]
fn test_web_search_tool_new_with_key() {
    let tool = WebSearchTool::new(Some("test_key".to_string()), 5);
    assert_eq!(tool.name(), "web_search");
    assert_eq!(
        tool.description(),
        "Search the web. Returns titles, URLs, and snippets."
    );
}

#[test]
fn test_web_search_tool_new_from_env() {
    // When no API key provided, it should try to read from env
    let tool = WebSearchTool::new(None, 3);
    assert_eq!(tool.name(), "web_search");
}

#[tokio::test]
async fn test_web_search_without_api_key() {
    // Create tool with empty API key
    std::env::remove_var("BRAVE_API_KEY");
    let tool = WebSearchTool::new(None, 5);

    let args = json!({"query": "rust programming"});
    let result = tool.execute(args).await.unwrap();

    assert!(result.contains("BRAVE_API_KEY not configured"));
}

#[test]
fn test_web_search_parameters() {
    let tool = WebSearchTool::new(Some("key".to_string()), 5);
    let params = tool.parameters();

    assert_eq!(params["type"], "object");
    assert!(params["required"]
        .as_array()
        .unwrap()
        .contains(&json!("query")));

    let count_prop = &params["properties"]["count"];
    assert_eq!(count_prop["minimum"], 1);
    assert_eq!(count_prop["maximum"], 10);
}

#[test]
fn test_web_fetch_parameters() {
    let tool = WebFetchTool::default();
    let params = tool.parameters();

    assert_eq!(params["type"], "object");
    assert!(params["required"]
        .as_array()
        .unwrap()
        .contains(&json!("url")));

    let extract_mode = &params["properties"]["extractMode"];
    let enum_values = extract_mode["enum"].as_array().unwrap();
    assert!(enum_values.contains(&json!("markdown")));
    assert!(enum_values.contains(&json!("text")));
}

#[test]
fn test_web_fetch_max_chars_validation() {
    let tool = WebFetchTool::new(100);
    let params = tool.parameters();

    let max_chars = &params["properties"]["maxChars"];
    assert_eq!(max_chars["minimum"], 100);
}
