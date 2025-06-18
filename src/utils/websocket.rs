//! WebSocket utility module for real-time market data and notifications

use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;
use futures_util::{SinkExt, StreamExt};
use anyhow::Result;

/// Basic WebSocket client for subscribing to real-time feeds
pub struct WebSocketClient {
    url: Url,
    max_retries: usize,
}

impl WebSocketClient {
    pub fn new(url: &str) -> Result<Self> {
        Ok(Self { url: Url::parse(url)?, max_retries: 5 })
    }

    /// Connect and run a message handler, with reconnection logic
    pub async fn run<F>(&self, mut handler: F) -> Result<()>
    where
        F: FnMut(Message) -> Result<()> + Send + 'static,
    {
        let mut attempts = 0;
        loop {
            match connect_async(&self.url).await {
                Ok((ws_stream, _)) => {
                    let (mut write, mut read) = ws_stream.split();
                    write.send(Message::Ping(vec![])).await?;
                    while let Some(msg) = read.next().await {
                        let msg = msg?;
                        handler(msg)?;
                    }
                    break;
                }
                Err(e) => {
                    attempts += 1;
                    if attempts > self.max_retries {
                        return Err(anyhow::anyhow!("WebSocket connection failed after {} attempts: {}", self.max_retries, e));
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
        Ok(())
    }

    /// Subscribe to a channel by sending a message after connect
    pub async fn subscribe<F>(&self, subscribe_msg: Message, mut handler: F) -> Result<()> 
    where
        F: FnMut(Message) -> Result<()> + Send + 'static,
    {
        let mut attempts = 0;
        loop {
            match connect_async(&self.url).await {
                Ok((ws_stream, _)) => {
                    let (mut write, mut read) = ws_stream.split();
                    write.send(subscribe_msg.clone()).await?;
                    while let Some(msg) = read.next().await {
                        let msg = msg?;
                        handler(msg)?;
                    }
                    break;
                }
                Err(e) => {
                    attempts += 1;
                    if attempts > self.max_retries {
                        return Err(anyhow::anyhow!("WebSocket subscribe failed after {} attempts: {}", self.max_retries, e));
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
        Ok(())
    }
}
