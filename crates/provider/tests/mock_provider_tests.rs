//! Mock Provider Tests
//!
//! Tests using mockall for the Provider trait to verify
//! that the trait can be properly mocked and used.

use async_trait::async_trait;
use mockall::mock;
use opensam_provider::{
    ChatParams, ChatResponse, Message, Provider, ProviderError, Tool, ToolChoice,
};
use serde_json::json;

// Create a mock implementation of the Provider trait
mock! {
    pub Provider {}

    #[async_trait]
    impl Provider for Provider {
        async fn chat(&self, params: ChatParams) -> Result<ChatResponse, ProviderError>;
        fn default_model(&self) -> String;
        fn is_configured(&self) -> bool;
    }
}

#[tokio::test]
async fn test_mock_provider_chat_returns_success() {
    let mut mock = MockProvider::new();

    // Set up expectations
    mock.expect_chat()
        .times(1)
        .returning(|_| Ok(ChatResponse::text("Hello from mock!")));

    // Use the mock
    let params = ChatParams::default();
    let response = mock.chat(params).await.unwrap();

    assert_eq!(response.content, Some("Hello from mock!".to_string()));
    assert!(!response.has_tool_calls());
}

#[tokio::test]
async fn test_mock_provider_chat_returns_error() {
    let mut mock = MockProvider::new();

    mock.expect_chat()
        .times(1)
        .returning(|_| Err(ProviderError::Api("Mock API error".to_string())));

    let params = ChatParams::default();
    let result = mock.chat(params).await;

    assert!(result.is_err());
    match result {
        Err(ProviderError::Api(msg)) => assert_eq!(msg, "Mock API error"),
        _ => panic!("Expected Api error"),
    }
}

#[tokio::test]
async fn test_mock_provider_chat_with_tool_calls() {
    let mut mock = MockProvider::new();

    mock.expect_chat()
        .times(1)
        .withf(|params| {
            // Verify that the params contain expected values
            params.messages.len() == 1 && params.messages[0].role == "user"
        })
        .returning(|_| {
            Ok(ChatResponse {
                content: Some("I'll help you with that".to_string()),
                tool_calls: vec![opensam_provider::ToolCall {
                    id: "mock_call_1".to_string(),
                    name: "mock_tool".to_string(),
                    arguments: json!({"arg": "value"}),
                }],
                finish_reason: "tool_calls".to_string(),
                usage: opensam_provider::Usage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                },
            })
        });

    let params = ChatParams {
        model: "test-model".to_string(),
        messages: vec![Message::user("Do something")],
        tools: vec![],
        max_tokens: 100,
        temperature: 0.5,
        tool_choice: ToolChoice::Auto,
    };

    let response = mock.chat(params).await.unwrap();

    assert!(response.has_tool_calls());
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].name, "mock_tool");
}

#[test]
fn test_mock_provider_default_model() {
    let mut mock = MockProvider::new();

    mock.expect_default_model()
        .times(1)
        .returning(|| "mock-model-v1".to_string());

    let model = mock.default_model();
    assert_eq!(model, "mock-model-v1");
}

#[test]
fn test_mock_provider_is_configured_true() {
    let mut mock = MockProvider::new();

    mock.expect_is_configured().times(1).returning(|| true);

    assert!(mock.is_configured());
}

#[test]
fn test_mock_provider_is_configured_false() {
    let mut mock = MockProvider::new();

    mock.expect_is_configured().times(1).returning(|| false);

    assert!(!mock.is_configured());
}

#[tokio::test]
async fn test_mock_provider_multiple_calls() {
    let mut mock = MockProvider::new();

    mock.expect_chat().times(3).returning(|params| {
        let content = params
            .messages
            .first()
            .and_then(|m| m.content.clone())
            .unwrap_or_default();
        Ok(ChatResponse::text(format!("Echo: {}", content)))
    });

    // Call multiple times
    for i in 0..3 {
        let params = ChatParams {
            model: "test".to_string(),
            messages: vec![Message::user(format!("Message {}", i))],
            tools: vec![],
            max_tokens: 100,
            temperature: 0.7,
            tool_choice: ToolChoice::Auto,
        };

        let response = mock.chat(params).await.unwrap();
        assert!(response
            .content
            .as_ref()
            .unwrap()
            .contains(&format!("Message {}", i)));
    }
}

#[tokio::test]
async fn test_mock_provider_chat_rate_limited() {
    let mut mock = MockProvider::new();

    mock.expect_chat()
        .times(1)
        .returning(|_| Err(ProviderError::RateLimited));

    let params = ChatParams::default();
    let result = mock.chat(params).await;

    assert!(matches!(result, Err(ProviderError::RateLimited)));
}

#[tokio::test]
async fn test_mock_provider_chat_no_api_key() {
    let mut mock = MockProvider::new();

    mock.expect_chat()
        .times(1)
        .returning(|_| Err(ProviderError::NoApiKey));

    let params = ChatParams::default();
    let result = mock.chat(params).await;

    assert!(matches!(result, Err(ProviderError::NoApiKey)));
}

#[tokio::test]
async fn test_mock_provider_with_complex_params() {
    let mut mock = MockProvider::new();

    mock.expect_chat()
        .times(1)
        .withf(|params| {
            params.model == "gpt-4"
                && params.max_tokens == 2048
                && params.temperature == 0.5
                && params.messages.len() == 2
                && matches!(params.tool_choice, ToolChoice::Auto)
        })
        .returning(|_| {
            Ok(ChatResponse {
                content: Some("Success!".to_string()),
                tool_calls: vec![],
                finish_reason: "stop".to_string(),
                usage: opensam_provider::Usage {
                    prompt_tokens: 100,
                    completion_tokens: 50,
                    total_tokens: 150,
                },
            })
        });

    let params = ChatParams {
        model: "gpt-4".to_string(),
        messages: vec![Message::system("You are helpful"), Message::user("Hello")],
        tools: vec![Tool::new(
            "test_tool",
            "A test tool",
            json!({"type": "object", "properties": {}}),
        )],
        max_tokens: 2048,
        temperature: 0.5,
        tool_choice: ToolChoice::Auto,
    };

    let response = mock.chat(params).await.unwrap();
    assert_eq!(response.content, Some("Success!".to_string()));
    assert_eq!(response.usage.total_tokens, 150);
}

#[tokio::test]
async fn test_mock_provider_different_responses_based_on_input() {
    let mut mock = MockProvider::new();

    mock.expect_chat().times(2).returning(|params| {
        let content = params
            .messages
            .first()
            .and_then(|m| m.content.clone())
            .unwrap_or_default();

        if content.contains("tool") {
            Ok(ChatResponse {
                content: None,
                tool_calls: vec![opensam_provider::ToolCall {
                    id: "call_1".to_string(),
                    name: "required_tool".to_string(),
                    arguments: json!({}),
                }],
                finish_reason: "tool_calls".to_string(),
                usage: opensam_provider::Usage::default(),
            })
        } else {
            Ok(ChatResponse::text("Direct response"))
        }
    });

    // First call - direct response
    let params1 = ChatParams {
        messages: vec![Message::user("Hello")],
        ..ChatParams::default()
    };
    let response1 = mock.chat(params1).await.unwrap();
    assert_eq!(response1.content, Some("Direct response".to_string()));
    assert!(!response1.has_tool_calls());

    // Second call - tool response
    let params2 = ChatParams {
        messages: vec![Message::user("Use a tool")],
        ..ChatParams::default()
    };
    let response2 = mock.chat(params2).await.unwrap();
    assert!(response2.has_tool_calls());
}

// Test using a struct that contains a Provider trait object
struct ProviderConsumer {
    provider: Box<dyn Provider>,
}

impl ProviderConsumer {
    async fn process_message(&self, message: &str) -> Result<String, ProviderError> {
        let params = ChatParams {
            model: "test-model".to_string(),
            messages: vec![Message::user(message)],
            tools: vec![],
            max_tokens: 100,
            temperature: 0.7,
            tool_choice: ToolChoice::Auto,
        };

        let response = self.provider.chat(params).await?;
        Ok(response.content.unwrap_or_default())
    }

    fn is_ready(&self) -> bool {
        self.provider.is_configured()
    }
}

#[tokio::test]
async fn test_mock_provider_in_consumer() {
    let mut mock = MockProvider::new();

    mock.expect_is_configured().times(1).returning(|| true);

    mock.expect_chat()
        .times(1)
        .returning(|_| Ok(ChatResponse::text("Processed!")));

    let consumer = ProviderConsumer {
        provider: Box::new(mock),
    };

    assert!(consumer.is_ready());

    let result = consumer.process_message("Hello").await.unwrap();
    assert_eq!(result, "Processed!");
}

#[tokio::test]
async fn test_mock_provider_invalid_response_error() {
    let mut mock = MockProvider::new();

    mock.expect_chat()
        .times(1)
        .returning(|_| Err(ProviderError::InvalidResponse));

    let params = ChatParams::default();
    let result = mock.chat(params).await;

    assert!(matches!(result, Err(ProviderError::InvalidResponse)));
}

#[tokio::test]
async fn test_mock_provider_json_error() {
    let mut mock = MockProvider::new();

    mock.expect_chat().times(1).returning(|_| {
        // Simulate a JSON parsing error
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        Err(ProviderError::Json(json_err))
    });

    let params = ChatParams::default();
    let result = mock.chat(params).await;

    assert!(matches!(result, Err(ProviderError::Json(_))));
}
