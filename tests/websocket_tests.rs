//! Integration tests for WebSocketClient

extern crate algotraderv2 as algotraderv2_rust;

use algotraderv2_rust::utils::websocket::WebSocketClient;

#[tokio::test]
async fn test_websocket_connect() {
    let url = "wss://echo.websocket.events";
    let client = WebSocketClient::new(url).expect("Failed to create client");
    let result = client.run(|_msg| Ok(())).await;
    assert!(result.is_ok());
}

// TODO: Add more tests for reconnection and message handling
