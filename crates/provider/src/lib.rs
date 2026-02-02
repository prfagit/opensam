//! SOLITON: LLM Provider Network
//!
//! Multi-node AI provider support for tactical operations.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use thiserror::Error;
use tracing::{debug, trace};

pub mod openrouter;

pub use openrouter::OpenRouterProvider;

/// SOLITON network errors
#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("SIGNAL LOST: {0}")]
    Request(#[from] reqwest::Error),

    #[error("DECRYPTION ERROR: {0}")]
    Json(#[from] serde_json::Error),

    #[error("NODE REJECTED: {0}")]
    Api(String),

    #[error("ACCESS DENIED: NO API KEY")]
    NoApiKey,

    #[error("CORRUPTED RESPONSE")]
    InvalidResponse,

    #[error("RATE LIMITED - RETREAT")]
    RateLimited,
}

pub type Result<T> = std::result::Result<T, ProviderError>;

/// Tool deployment request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// SOLITON response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default)]
    pub finish_reason: String,
    #[serde(default)]
    pub usage: Usage,
}

impl ChatResponse {
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }

    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            tool_calls: Vec::new(),
            finish_reason: "stop".to_string(),
            usage: Usage::default(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: Some(message.into()),
            tool_calls: Vec::new(),
            finish_reason: "error".to_string(),
            usage: Usage::default(),
        }
    }
}

/// Resource consumption
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Transmission log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallDef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn tool(
        call_id: impl Into<String>,
        name: impl Into<String>,
        result: impl Into<String>,
    ) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(result.into()),
            tool_calls: None,
            tool_call_id: Some(call_id.into()),
            name: Some(name.into()),
        }
    }
}

/// Tool call specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDef {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

impl ToolCallDef {
    pub fn new(id: impl Into<String>, name: impl Into<String>, arguments: Value) -> Self {
        Self {
            id: id.into(),
            call_type: "function".to_string(),
            function: FunctionCall {
                name: name.into(),
                arguments,
            },
        }
    }
}

/// Function parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: Value,
}

/// Tool specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDef,
}

impl Tool {
    pub fn new(name: impl Into<String>, description: impl Into<String>, parameters: Value) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: name.into(),
                description: description.into(),
                parameters,
            },
        }
    }
}

/// Function schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// Transmission parameters
#[derive(Debug, Clone)]
pub struct ChatParams {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<Tool>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub tool_choice: ToolChoice,
}

impl Default for ChatParams {
    fn default() -> Self {
        Self {
            model: String::new(),
            messages: Vec::new(),
            tools: Vec::new(),
            max_tokens: 4096,
            temperature: 0.7,
            tool_choice: ToolChoice::Auto,
        }
    }
}

/// Tool selection mode
#[derive(Debug, Clone)]
pub enum ToolChoice {
    Auto,
    Required(String),
    None,
}

/// SOLITON network node
#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat(&self, params: ChatParams) -> Result<ChatResponse>;
    fn default_model(&self) -> String;
    fn is_configured(&self) -> bool;
}

/// Build JSON schema
pub fn object_schema(properties: Vec<(String, String, bool)>) -> Value {
    let mut props = serde_json::Map::new();
    let mut required = Vec::new();

    for (name, description, is_required) in properties {
        props.insert(
            name.clone(),
            serde_json::json!({
                "type": "string",
                "description": description
            }),
        );
        if is_required {
            required.push(name);
        }
    }

    serde_json::json!({
        "type": "object",
        "properties": props,
        "required": required
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ========== ProviderError Tests ==========

    #[test]
    fn test_provider_error_display() {
        let err = ProviderError::NoApiKey;
        assert_eq!(err.to_string(), "ACCESS DENIED: NO API KEY");

        let err = ProviderError::Api("test error".to_string());
        assert_eq!(err.to_string(), "NODE REJECTED: test error");

        let err = ProviderError::InvalidResponse;
        assert_eq!(err.to_string(), "CORRUPTED RESPONSE");

        let err = ProviderError::RateLimited;
        assert_eq!(err.to_string(), "RATE LIMITED - RETREAT");
    }

    #[test]
    fn test_provider_error_from_reqwest() {
        // Note: We can't easily create a reqwest::Error, but we can verify the From trait exists
        // by checking the error type implements the expected traits
        fn assert_provider_error_traits<T: std::error::Error + std::fmt::Debug>() {}
        assert_provider_error_traits::<ProviderError>();
    }

    // ========== ChatResponse Tests ==========

    #[test]
    fn test_chat_response_text_builder() {
        let response = ChatResponse::text("Hello, world!");
        assert_eq!(response.content, Some("Hello, world!".to_string()));
        assert!(response.tool_calls.is_empty());
        assert_eq!(response.finish_reason, "stop");
    }

    #[test]
    fn test_chat_response_error_builder() {
        let response = ChatResponse::error("Something went wrong");
        assert_eq!(response.content, Some("Something went wrong".to_string()));
        assert!(response.tool_calls.is_empty());
        assert_eq!(response.finish_reason, "error");
    }

    #[test]
    fn test_chat_response_has_tool_calls() {
        let response_without_tools = ChatResponse::text("Hello");
        assert!(!response_without_tools.has_tool_calls());

        let response_with_tools = ChatResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_1".to_string(),
                name: "test_tool".to_string(),
                arguments: json!({}),
            }],
            finish_reason: "tool_calls".to_string(),
            usage: Usage::default(),
        };
        assert!(response_with_tools.has_tool_calls());
    }

    #[test]
    fn test_chat_response_default_usage() {
        let response = ChatResponse::text("test");
        assert_eq!(response.usage.prompt_tokens, 0);
        assert_eq!(response.usage.completion_tokens, 0);
        assert_eq!(response.usage.total_tokens, 0);
    }

    // ========== Usage Tests ==========

    #[test]
    fn test_usage_default() {
        let usage = Usage::default();
        assert_eq!(usage.prompt_tokens, 0);
        assert_eq!(usage.completion_tokens, 0);
        assert_eq!(usage.total_tokens, 0);
    }

    // ========== Message Tests ==========

    #[test]
    fn test_message_system() {
        let msg = Message::system("You are a helpful assistant");
        assert_eq!(msg.role, "system");
        assert_eq!(msg.content, Some("You are a helpful assistant".to_string()));
        assert!(msg.tool_calls.is_none());
        assert!(msg.tool_call_id.is_none());
        assert!(msg.name.is_none());
    }

    #[test]
    fn test_message_user() {
        let msg = Message::user("What's the weather?");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, Some("What's the weather?".to_string()));
        assert!(msg.tool_calls.is_none());
        assert!(msg.tool_call_id.is_none());
        assert!(msg.name.is_none());
    }

    #[test]
    fn test_message_assistant() {
        let msg = Message::assistant("The weather is sunny");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, Some("The weather is sunny".to_string()));
        assert!(msg.tool_calls.is_none());
        assert!(msg.tool_call_id.is_none());
        assert!(msg.name.is_none());
    }

    #[test]
    fn test_message_tool() {
        let msg = Message::tool("call_123", "get_weather", "{\"temperature\": 72}");
        assert_eq!(msg.role, "tool");
        assert_eq!(msg.content, Some("{\"temperature\": 72}".to_string()));
        assert!(msg.tool_calls.is_none());
        assert_eq!(msg.tool_call_id, Some("call_123".to_string()));
        assert_eq!(msg.name, Some("get_weather".to_string()));
    }

    #[test]
    fn test_message_builder_with_string() {
        let s = String::from("test content");
        let msg = Message::user(s);
        assert_eq!(msg.content, Some("test content".to_string()));
    }

    // ========== ToolCallDef Tests ==========

    #[test]
    fn test_tool_call_def_new() {
        let args = json!({"location": "NYC"});
        let def = ToolCallDef::new("call_1", "get_weather", args.clone());

        assert_eq!(def.id, "call_1");
        assert_eq!(def.call_type, "function");
        assert_eq!(def.function.name, "get_weather");
        assert_eq!(def.function.arguments, args);
    }

    #[test]
    fn test_tool_call_def_with_different_types() {
        // Test with &str
        let def1 = ToolCallDef::new("id1", "func1", json!({}));
        assert_eq!(def1.id, "id1");
        assert_eq!(def1.function.name, "func1");

        // Test with String
        let def2 = ToolCallDef::new(String::from("id2"), String::from("func2"), json!({}));
        assert_eq!(def2.id, "id2");
        assert_eq!(def2.function.name, "func2");
    }

    // ========== Tool Tests ==========

    #[test]
    fn test_tool_new() {
        let params = json!({
            "type": "object",
            "properties": {
                "location": {"type": "string"}
            }
        });
        let tool = Tool::new("get_weather", "Get weather information", params.clone());

        assert_eq!(tool.tool_type, "function");
        assert_eq!(tool.function.name, "get_weather");
        assert_eq!(tool.function.description, "Get weather information");
        assert_eq!(tool.function.parameters, params);
    }

    #[test]
    fn test_tool_with_different_types() {
        let params = json!({});

        // Test with &str
        let tool1 = Tool::new("func1", "description1", params.clone());
        assert_eq!(tool1.function.name, "func1");
        assert_eq!(tool1.function.description, "description1");

        // Test with String
        let tool2 = Tool::new(
            String::from("func2"),
            String::from("description2"),
            params.clone(),
        );
        assert_eq!(tool2.function.name, "func2");
        assert_eq!(tool2.function.description, "description2");
    }

    // ========== ChatParams Tests ==========

    #[test]
    fn test_chat_params_default() {
        let params = ChatParams::default();
        assert_eq!(params.model, "");
        assert!(params.messages.is_empty());
        assert!(params.tools.is_empty());
        assert_eq!(params.max_tokens, 4096);
        assert_eq!(params.temperature, 0.7);
        match params.tool_choice {
            ToolChoice::Auto => (), // expected
            _ => panic!("Expected ToolChoice::Auto"),
        }
    }

    #[test]
    fn test_chat_params_with_values() {
        let params = ChatParams {
            model: "gpt-4".to_string(),
            messages: vec![Message::user("Hello")],
            tools: vec![Tool::new("test", "test desc", json!({}))],
            max_tokens: 2048,
            temperature: 0.5,
            tool_choice: ToolChoice::Required("test_tool".to_string()),
        };

        assert_eq!(params.model, "gpt-4");
        assert_eq!(params.messages.len(), 1);
        assert_eq!(params.tools.len(), 1);
        assert_eq!(params.max_tokens, 2048);
        assert_eq!(params.temperature, 0.5);
    }

    // ========== ToolChoice Tests ==========

    #[test]
    fn test_tool_choice_variants() {
        let auto = ToolChoice::Auto;
        let required = ToolChoice::Required("specific_tool".to_string());
        let none = ToolChoice::None;

        // Just verify they can be created and match correctly
        match auto {
            ToolChoice::Auto => (),
            _ => panic!("Expected Auto"),
        }

        match required {
            ToolChoice::Required(name) => assert_eq!(name, "specific_tool"),
            _ => panic!("Expected Required"),
        }

        match none {
            ToolChoice::None => (),
            _ => panic!("Expected None"),
        }
    }

    // ========== object_schema Tests ==========

    #[test]
    fn test_object_schema_empty() {
        let schema = object_schema(vec![]);
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].as_object().unwrap().is_empty());
        assert!(schema["required"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_object_schema_single_required() {
        let schema = object_schema(vec![(
            "name".to_string(),
            "The user's name".to_string(),
            true,
        )]);

        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["name"].is_object());
        assert_eq!(schema["properties"]["name"]["type"], "string");
        assert_eq!(
            schema["properties"]["name"]["description"],
            "The user's name"
        );

        let required = schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "name");
    }

    #[test]
    fn test_object_schema_multiple_mixed_required() {
        let schema = object_schema(vec![
            ("name".to_string(), "The user's name".to_string(), true),
            ("age".to_string(), "The user's age".to_string(), false),
            ("email".to_string(), "The user's email".to_string(), true),
        ]);

        let props = schema["properties"].as_object().unwrap();
        assert_eq!(props.len(), 3);
        assert!(props.contains_key("name"));
        assert!(props.contains_key("age"));
        assert!(props.contains_key("email"));

        let required = schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 2);
        assert!(required.contains(&json!("name")));
        assert!(required.contains(&json!("email")));
        assert!(!required.contains(&json!("age")));
    }

    #[test]
    fn test_object_schema_with_str_refs() {
        // Test that it works with &str by converting to String
        let props: Vec<(String, String, bool)> =
            vec![("city".to_string(), "City name".to_string(), true)];
        let schema = object_schema(props);
        assert!(schema["properties"]["city"].is_object());
    }

    // ========== Serialization Tests ==========

    #[test]
    fn test_message_serialization() {
        let msg = Message::user("Hello");
        let json_str = serde_json::to_string(&msg).unwrap();
        assert!(json_str.contains("\"role\":\"user\""));
        assert!(json_str.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_chat_response_serialization() {
        let response = ChatResponse::text("Hello!");
        let json_str = serde_json::to_string(&response).unwrap();
        assert!(json_str.contains("\"content\":\"Hello!\""));
        assert!(json_str.contains("\"finish_reason\":\"stop\""));
    }

    #[test]
    fn test_tool_serialization() {
        let tool = Tool::new("get_weather", "Get weather", json!({}));
        let json_str = serde_json::to_string(&tool).unwrap();
        assert!(json_str.contains("\"type\":\"function\""));
        assert!(json_str.contains("\"name\":\"get_weather\""));
    }

    #[test]
    fn test_message_deserialization() {
        let json_str = r#"{"role":"assistant","content":"Hi there"}"#;
        let msg: Message = serde_json::from_str(json_str).unwrap();
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, Some("Hi there".to_string()));
    }

    #[test]
    fn test_chat_response_with_tool_calls_serialization() {
        let response = ChatResponse {
            content: Some("Calling tool".to_string()),
            tool_calls: vec![ToolCall {
                id: "call_1".to_string(),
                name: "get_weather".to_string(),
                arguments: json!({"location": "NYC"}),
            }],
            finish_reason: "tool_calls".to_string(),
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            },
        };

        let json_str = serde_json::to_string(&response).unwrap();
        let deserialized: ChatResponse = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.content, response.content);
        assert_eq!(deserialized.tool_calls.len(), 1);
        assert_eq!(deserialized.tool_calls[0].id, "call_1");
        assert_eq!(deserialized.tool_calls[0].name, "get_weather");
    }

    // ========== ToolCall Tests ==========

    #[test]
    fn test_tool_call_creation() {
        let tool_call = ToolCall {
            id: "call_123".to_string(),
            name: "search".to_string(),
            arguments: json!({"query": "rust programming"}),
        };

        assert_eq!(tool_call.id, "call_123");
        assert_eq!(tool_call.name, "search");
        assert_eq!(tool_call.arguments, json!({"query": "rust programming"}));
    }

    #[test]
    fn test_tool_call_serialization() {
        let tool_call = ToolCall {
            id: "call_123".to_string(),
            name: "search".to_string(),
            arguments: json!({"query": "test"}),
        };

        let json_str = serde_json::to_string(&tool_call).unwrap();
        assert!(json_str.contains("\"id\":\"call_123\""));
        assert!(json_str.contains("\"name\":\"search\""));
    }

    // ========== FunctionDef Tests ==========

    #[test]
    fn test_function_def_creation() {
        let func_def = FunctionDef {
            name: "calculate".to_string(),
            description: "Perform a calculation".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        };

        assert_eq!(func_def.name, "calculate");
        assert_eq!(func_def.description, "Perform a calculation");
    }
}
