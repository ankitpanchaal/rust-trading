use binance::websockets::*;
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::{mpsc, RwLock};

use crate::error::AppError;

// Represents different types of WebSocket events that can be received
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum WebSocketEvent {
    Ticker {
        symbol: String,
        price: String,
        high: String,
        low: String,
        volume: String,
        quote_volume: String,
    },
    Kline {
        symbol: String,
        interval: String,
        open_time: u64,
        close_time: u64,
        open: String,
        high: String,
        low: String,
        close: String,
        volume: String,
    },
    Depth {
        symbol: String,
        bids: Vec<(String, String)>,
        asks: Vec<(String, String)>,
    },
}

// Service to manage Binance WebSocket connections and handle events
#[derive(Clone)]
pub struct BinanceWebSocketService {
    event_sender: mpsc::Sender<WebSocketEvent>, // Channel to send WebSocket events to consumers
    active_connections: Arc<RwLock<std::collections::HashMap<String, Arc<AtomicBool>>>>, // Tracks active WebSocket connections
}

impl BinanceWebSocketService {
    // Creates a new instance of the service and returns it along with a receiver for WebSocket events
    pub fn new() -> (Self, mpsc::Receiver<WebSocketEvent>) {
        let (tx, rx) = mpsc::channel::<WebSocketEvent>(100);

        let service = Self {
            event_sender: tx,
            active_connections: Arc::new(RwLock::new(std::collections::HashMap::new())),
        };

        (service, rx)
    }

    // Subscribes to the ticker stream for a specific symbol
    pub async fn subscribe_ticker(&self, symbol: &str) -> Result<String, AppError> {
        let connection_id = format!("ticker_{}", symbol);

        // Check if the subscription already exists
        {
            let connections = self.active_connections.read().await;
            if connections.contains_key(&connection_id) {
                return Ok(connection_id);
            }
        }

        // Create a new subscription and track its state
        let keep_running = Arc::new(AtomicBool::new(true));
        let endpoint = format!("{}@ticker", symbol.to_lowercase());
        let event_sender = self.event_sender.clone();
        let keep_running_clone = keep_running.clone();

        {
            let mut connections = self.active_connections.write().await;
            connections.insert(connection_id.clone(), keep_running);
        }

        // Start a new task to handle WebSocket events

        tokio::task::spawn_blocking(move || {
            let mut web_socket = WebSockets::new(move |event: WebsocketEvent| {
                match event {
                    WebsocketEvent::DayTicker(ticker) => {
                        let ws_event = WebSocketEvent::Ticker {
                            symbol: ticker.symbol,
                            price: ticker.current_close,
                            high: ticker.high,
                            low: ticker.low,
                            volume: ticker.volume,
                            quote_volume: ticker.quote_volume,
                        };

                        let _ = event_sender.try_send(ws_event);
                    }
                    _ => (),
                };

                Ok(())
            });

            if let Ok(_) = web_socket.connect(&endpoint) {
                let _ = web_socket.event_loop(&keep_running_clone);
            }
        });

        Ok(connection_id)
    }

    // Subscribes to the kline (candlestick) stream for a specific symbol and interval
    pub async fn subscribe_kline(&self, symbol: &str, interval: &str) -> Result<String, AppError> {
        let connection_id = format!("kline_{}_{}", symbol, interval);

        // Check if the subscription already exists
        {
            let connections = self.active_connections.read().await;
            if connections.contains_key(&connection_id) {
                return Ok(connection_id);
            }
        }

        // Create a new subscription and track its state
        let keep_running = Arc::new(AtomicBool::new(true));
        let endpoint = format!("{}@kline_{}", symbol.to_lowercase(), interval);
        let event_sender = self.event_sender.clone();
        let keep_running_clone = keep_running.clone();

        {
            let mut connections = self.active_connections.write().await;
            connections.insert(connection_id.clone(), keep_running);
        }

        // Start a new task to handle WebSocket events
        tokio::spawn(async move {
            let mut web_socket = WebSockets::new(move |event: WebsocketEvent| {
                match event {
                    WebsocketEvent::Kline(kline_event) => {
                        let kline = kline_event.kline;
                        let ws_event = WebSocketEvent::Kline {
                            symbol: kline.symbol,
                            interval: kline.interval,
                            open_time: kline.open_time as u64,
                            close_time: kline.close_time as u64,
                            open: kline.open,
                            high: kline.high,
                            low: kline.low,
                            close: kline.close,
                            volume: kline.volume,
                        };

                        let _ = event_sender.try_send(ws_event);
                    }
                    _ => (),
                };

                Ok(())
            });

            if let Ok(_) = web_socket.connect(&endpoint) {
                let _ = web_socket.event_loop(&keep_running_clone);
            }
        });

        Ok(connection_id)
    }

    // Subscribes to the depth (order book) stream for a specific symbol
    pub async fn subscribe_depth(&self, symbol: &str) -> Result<String, AppError> {
        let connection_id = format!("depth_{}", symbol);

        // Check if the subscription already exists
        {
            let connections = self.active_connections.read().await;
            if connections.contains_key(&connection_id) {
                return Ok(connection_id);
            }
        }

        // Create a new subscription and track its state
        let keep_running = Arc::new(AtomicBool::new(true));
        let endpoint = format!("{}@depth@100ms", symbol.to_lowercase());
        let event_sender = self.event_sender.clone();
        let keep_running_clone = keep_running.clone();

        {
            let mut connections = self.active_connections.write().await;
            connections.insert(connection_id.clone(), keep_running);
        }

        // Start a new task to handle WebSocket events
        tokio::spawn(async move {
            let mut web_socket = WebSockets::new(move |event: WebsocketEvent| {
                match event {
                    WebsocketEvent::DepthOrderBook(depth) => {
                        let bids: Vec<(String, String)> = depth
                            .bids
                            .iter()
                            .map(|bid| (bid.price.to_string(), bid.qty.to_string()))
                            .collect();

                        let asks: Vec<(String, String)> = depth
                            .asks
                            .iter()
                            .map(|ask| (ask.price.to_string(), ask.qty.to_string()))
                            .collect();

                        let ws_event = WebSocketEvent::Depth {
                            symbol: depth.symbol,
                            bids,
                            asks,
                        };

                        let _ = event_sender.try_send(ws_event);
                    }
                    _ => (),
                };

                Ok(())
            });

            if let Ok(_) = web_socket.connect(&endpoint) {
                let _ = web_socket.event_loop(&keep_running_clone);
            }
        });

        Ok(connection_id)
    }

    // Unsubscribes from a WebSocket stream by its connection ID
    pub async fn unsubscribe(&self, connection_id: &str) -> Result<(), AppError> {
        let mut connections = self.active_connections.write().await;

        if let Some(keep_running) = connections.get(connection_id) {
            keep_running.store(false, Ordering::Relaxed); // Stop the WebSocket event loop
            connections.remove(connection_id); // Remove the connection from the active list
            Ok(())
        } else {
            Err(AppError::NotFoundError(format!(
                "Connection {} not found",
                connection_id
            )))
        }
    }
}
