use std::process::Command;
use vox_tui::config::{Config, MiniMaxConfig, Provider, ProviderModels, StepFunConfig};
use vox_tui::provider::{create_provider, Message};

#[test]
fn test_binary_runs() {
    let output = Command::new(env!("CARGO_BIN_EXE_vox"))
        .arg("--help")
        .output()
        .expect("Failed to run vox binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("vox")
            || stdout.contains("Multi-provider")
            || stderr.contains("terminal")
            || output.status.code() == Some(1),
        "Unexpected output: stdout={:?}, stderr={:?}",
        stdout,
        stderr
    );
}

#[tokio::test]
async fn test_stepfun_chat_success() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "test-model",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello from mock!"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        }"#,
        )
        .create_async()
        .await;

    let config = Config {
        default_provider: Provider::StepFun,
        stepfun: Some(StepFunConfig {
            api_key: "test-key".into(),
            base_url: Some(server.url()),
            model: Some("test-model".into()),
            models: ProviderModels::default(),
        }),
        minimax: None,
        theme: None,
    };

    let provider = create_provider(&config).unwrap();
    let result = provider.chat(&[Message::user("hello")]).await;

    assert!(result.is_ok(), "Expected success, got: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.content, "Hello from mock!");
    assert_eq!(response.model, "test-model");

    mock.assert_async().await;
}

#[tokio::test]
async fn test_stepfun_chat_error_5xx() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/chat/completions")
        .with_status(500)
        .with_body("Internal Server Error")
        .expect(4)
        .create_async()
        .await;

    let config = Config {
        default_provider: Provider::StepFun,
        stepfun: Some(StepFunConfig {
            api_key: "test-key".into(),
            base_url: Some(server.url()),
            model: Some("test-model".into()),
            models: ProviderModels::default(),
        }),
        minimax: None,
        theme: None,
    };

    let provider = create_provider(&config).unwrap();
    let result = provider.chat(&[Message::user("hello")]).await;

    assert!(result.is_err(), "Expected error for 5xx response");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("500") || err.contains("API error"),
        "Expected API error in message, got: {}",
        err
    );

    mock.assert_async().await;
}

#[tokio::test]
async fn test_minimax_chat_success() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/text/chat")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "reply": "Hello from MiniMax mock!",
            "model": "test-model",
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        }"#,
        )
        .create_async()
        .await;

    let config = Config {
        default_provider: Provider::MiniMax,
        stepfun: None,
        minimax: Some(MiniMaxConfig {
            api_key: "test-key".into(),
            group_id: Some("test-group".into()),
            base_url: Some(server.url()),
            model: Some("test-model".into()),
            models: ProviderModels::default(),
        }),
        theme: None,
    };

    let provider = create_provider(&config).unwrap();
    let result = provider.chat(&[Message::user("hello")]).await;

    assert!(result.is_ok(), "Expected success, got: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.content, "Hello from MiniMax mock!");
    assert_eq!(response.model, "test-model");

    mock.assert_async().await;
}

#[tokio::test]
async fn test_minimax_image_generation_success() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/image_generation")
        .match_query(mockito::Matcher::UrlEncoded(
            "GroupId".into(),
            "test-group".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "data": {
                "image_urls": [
                    {"image_url": "https://example.com/image1.png"}
                ]
            }
        }"#,
        )
        .create_async()
        .await;

    let config = Config {
        default_provider: Provider::MiniMax,
        stepfun: None,
        minimax: Some(MiniMaxConfig {
            api_key: "test-key".into(),
            group_id: Some("test-group".into()),
            base_url: Some(server.url()),
            model: Some("test-model".into()),
            models: ProviderModels::default(),
        }),
        theme: None,
    };

    let provider = create_provider(&config).unwrap();
    let result = provider.image_generate("a cute cat", 1, "1:1").await;

    assert!(result.is_ok(), "Expected success, got: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.urls, vec!["https://example.com/image1.png"]);

    mock.assert_async().await;
}
