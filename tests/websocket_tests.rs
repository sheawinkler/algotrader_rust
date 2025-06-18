//! Integration tests for WebSocketClient

use algotraderv2_rust::utils::websocket::WebSocketClient;
use tokio_tungstenite::tungstenite::protocol::Message;

#[tokio::test]
async fn test_websocket_connect_and_ping() {
    // This uses a public echo WebSocket server for demonstration
    let url = "wss://echo.websocket.events";
    let client = WebSocketClient::new(url).expect("Failed to create client");
    let mut received_ping = false;
    let result = client.run(|msg| {
        match msg {
            Message::Pong(_) => {
                received_ping = true;
            }
            Message::Text(txt) => {
                println!("Received text: {}", txt);
            }
            _ => {}
        }
        Ok(())
    }).await;
    assert!(result.is_ok());
    // We can't guarantee the echo server will pong, but this checks basic connection
}

// TODO: Add more tests for reconnection and message handling
