use anyhow::{bail, Result};
use backoff::ExponentialBackoff;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{interval, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::api::progress::{ProgressEvent, ChannelType};

#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub url: String,
    pub reconnect_max_attempts: u32,
    pub reconnect_delay_ms: u64,
    pub ping_interval_secs: u64,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            url: "ws://localhost:8080/api/v1/ws".to_string(),
            reconnect_max_attempts: 5,
            reconnect_delay_ms: 1000,
            ping_interval_secs: 30,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ClientMessage {
    Subscribe {
        channel_type: ChannelType,
        id: Uuid,
    },
    Unsubscribe {
        channel_type: ChannelType,
        id: Uuid,
    },
    Ping,
}

#[derive(Debug, Deserialize)]
pub struct ServerMessage {
    pub message_type: ServerMessageType,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerMessageType {
    Connected,
    Subscribed,
    Unsubscribed,
    Progress,
    Error,
    Pong,
}

#[derive(Debug, Deserialize)]
pub struct WebSocketMessage {
    pub id: Uuid,
    pub event: ProgressEvent,
    pub user_id: Uuid,
    pub sequence: u64,
}

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

pub struct ProgressWebSocket {
    config: WebSocketConfig,
    token: String,
    ws_stream: Option<Arc<Mutex<WsStream>>>,
    event_receiver: mpsc::UnboundedReceiver<ProgressEvent>,
    event_sender: mpsc::UnboundedSender<ProgressEvent>,
    command_sender: Option<mpsc::UnboundedSender<ClientMessage>>,
    subscriptions: Arc<RwLock<Vec<(ChannelType, Uuid)>>>,
    is_connected: Arc<RwLock<bool>>,
}

impl ProgressWebSocket {
    pub fn new(config: WebSocketConfig, token: String) -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        Self {
            config,
            token,
            ws_stream: None,
            event_receiver,
            event_sender,
            command_sender: None,
            subscriptions: Arc::new(RwLock::new(Vec::new())),
            is_connected: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting to WebSocket at {}", self.config.url);

        let url_with_token = format!("{}?token={}", self.config.url, self.token);
        debug!("Full WebSocket URL: {}", url_with_token.replace(&self.token, "***"));

        let (ws_stream, response) = connect_async(url_with_token).await?;
        debug!("WebSocket connected with response status: {}", response.status());
        let ws_stream = Arc::new(Mutex::new(ws_stream));
        self.ws_stream = Some(ws_stream.clone());

        *self.is_connected.write().await = true;
        info!("WebSocket connection established successfully");

        let (cmd_sender, cmd_receiver) = mpsc::unbounded_channel();
        self.command_sender = Some(cmd_sender.clone());

        // Spawn message handler task
        self.spawn_message_handler(ws_stream.clone(), cmd_receiver).await;

        // Spawn ping task
        self.spawn_ping_task(cmd_sender.clone()).await;

        // Resubscribe to existing channels
        self.resubscribe().await?;

        info!("WebSocket connected successfully");
        Ok(())
    }

    async fn spawn_message_handler(
        &self,
        ws_stream: Arc<Mutex<WsStream>>,
        mut cmd_receiver: mpsc::UnboundedReceiver<ClientMessage>,
    ) {
        let event_sender = self.event_sender.clone();
        let is_connected = self.is_connected.clone();

        tokio::spawn(async move {
            loop {
                let mut ws = ws_stream.lock().await;

                tokio::select! {
                    // Handle incoming WebSocket messages
                    msg = ws.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                debug!("Received WebSocket text message: {}", text);
                                if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                                    debug!("Parsed server message: {:?}", server_msg.message_type);
                                    match server_msg.message_type {
                                        ServerMessageType::Progress => {
                                            if let Ok(ws_msg) = serde_json::from_value::<WebSocketMessage>(server_msg.data) {
                                                let _ = event_sender.send(ws_msg.event);
                                            }
                                        }
                                        ServerMessageType::Error => {
                                            error!("Server error: {:?}", server_msg.data);
                                        }
                                        _ => {
                                            debug!("Received message: {:?}", server_msg.message_type);
                                        }
                                    }
                                }
                            }
                            Some(Ok(Message::Close(_))) => {
                                info!("WebSocket closed by server");
                                *is_connected.write().await = false;
                                break;
                            }
                            Some(Err(e)) => {
                                error!("WebSocket error: {}", e);
                                *is_connected.write().await = false;
                                break;
                            }
                            None => {
                                warn!("WebSocket stream ended");
                                *is_connected.write().await = false;
                                break;
                            }
                            _ => {}
                        }
                    }

                    // Handle outgoing commands
                    cmd = cmd_receiver.recv() => {
                        if let Some(command) = cmd {
                            debug!("Sending WebSocket command: {:?}", command);
                            let msg = serde_json::to_string(&command)?;
                            debug!("Serialized WebSocket message: {}", msg);
                            if let Err(e) = ws.send(Message::Text(msg.clone())).await {
                                error!("Failed to send command: {}", e);
                                *is_connected.write().await = false;
                            } else {
                                info!("Successfully sent WebSocket message: {}", msg);
                            }
                        }
                    }
                }
            }

            Ok::<(), anyhow::Error>(())
        });
    }

    async fn spawn_ping_task(&self, cmd_sender: mpsc::UnboundedSender<ClientMessage>) {
        let interval_secs = self.config.ping_interval_secs;
        let is_connected = self.is_connected.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));

            loop {
                ticker.tick().await;

                if !*is_connected.read().await {
                    break;
                }

                if cmd_sender.send(ClientMessage::Ping).is_err() {
                    break;
                }
            }
        });
    }

    pub async fn subscribe_upload(&mut self, document_id: Uuid) -> Result<()> {
        self.subscribe(ChannelType::Upload, document_id).await
    }

    pub async fn subscribe_analysis(&mut self, analysis_id: Uuid) -> Result<()> {
        self.subscribe(ChannelType::Analysis, analysis_id).await
    }

    pub async fn subscribe_batch(&mut self, batch_id: Uuid) -> Result<()> {
        self.subscribe(ChannelType::Batch, batch_id).await
    }

    async fn subscribe(&mut self, channel_type: ChannelType, id: Uuid) -> Result<()> {
        debug!("Subscribe called for {:?} with ID: {}", channel_type, id);

        if !*self.is_connected.read().await {
            debug!("WebSocket not connected, connecting now...");
            self.connect().await?;
        }

        let channel_type_clone = channel_type.clone();
        let message = ClientMessage::Subscribe {
            channel_type: channel_type_clone.clone(),
            id,
        };
        debug!("Created subscribe message: {:?}", message);

        if let Some(sender) = &self.command_sender {
            debug!("Sending message through command channel...");
            sender.send(message)?;
            self.subscriptions.write().await.push((channel_type_clone.clone(), id));
            info!("Subscribed to {:?} channel for {}", channel_type_clone, id);
        } else {
            error!("WebSocket command_sender not available!");
            bail!("WebSocket not connected");
        }

        Ok(())
    }

    pub async fn unsubscribe(&mut self, channel_type: ChannelType, id: Uuid) -> Result<()> {
        if let Some(sender) = &self.command_sender {
            let channel_type_clone = channel_type.clone();
            let message = ClientMessage::Unsubscribe {
                channel_type: channel_type_clone.clone(),
                id,
            };
            sender.send(message)?;

            let mut subs = self.subscriptions.write().await;
            subs.retain(|(ct, i)| ct != &channel_type_clone || i != &id);

            info!("Unsubscribed from {:?} channel for {}", channel_type_clone, id);
        }

        Ok(())
    }

    pub async fn next_event(&mut self) -> Option<ProgressEvent> {
        self.event_receiver.recv().await
    }

    async fn resubscribe(&mut self) -> Result<()> {
        let subs = self.subscriptions.read().await.clone();
        for (channel_type, id) in subs {
            let message = ClientMessage::Subscribe {
                channel_type,
                id,
            };

            if let Some(sender) = &self.command_sender {
                sender.send(message)?;
            }
        }
        Ok(())
    }

    pub async fn handle_reconnect(&mut self) -> Result<()> {
        warn!("Attempting to reconnect WebSocket");

        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(60)),
            initial_interval: Duration::from_millis(self.config.reconnect_delay_ms),
            ..Default::default()
        };

        let mut attempts = 0;

        loop {
            if attempts >= self.config.reconnect_max_attempts {
                bail!("Max reconnection attempts reached");
            }

            attempts += 1;
            info!("Reconnection attempt {}/{}", attempts, self.config.reconnect_max_attempts);

            match self.connect().await {
                Ok(_) => {
                    info!("Successfully reconnected");
                    return Ok(());
                }
                Err(e) => {
                    warn!("Reconnection failed: {}", e);
                    if attempts < self.config.reconnect_max_attempts {
                        tokio::time::sleep(backoff.initial_interval * attempts).await;
                    }
                }
            }
        }
    }

    pub async fn disconnect(&mut self) {
        *self.is_connected.write().await = false;

        if let Some(ws_stream) = &self.ws_stream {
            let mut ws = ws_stream.lock().await;
            let _ = ws.close(None).await;
        }

        self.ws_stream = None;
        self.command_sender = None;
        info!("WebSocket disconnected");
    }

    pub async fn is_connected(&self) -> bool {
        *self.is_connected.read().await
    }
}