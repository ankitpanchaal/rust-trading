use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use validator::Validate;

use super::{
    market_service::BinanceMarketService, model::*, user_service::BinanceUserService,
    websocket_service::BinanceWebSocketService,
};

// Market handlers
pub async fn get_price(
    State(service): State<BinanceMarketService>,
    Path(symbol): Path<String>,
) -> Result<Json<MarketPriceResponse>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_ticker_price(&symbol).await {
        Ok((price, timestamp)) => {
            let response = MarketPriceResponse {
                symbol,
                price,
                timestamp,
            };
            Ok(Json(response))
        }
        Err(e) => {
            let error_response = ErrorResponse {
                error: format!("Failed to fetch market data: {}", e),
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

pub async fn get_price_post(
    State(service): State<BinanceMarketService>,
    Json(request): Json<MarketPriceRequest>,
) -> Result<Json<MarketPriceResponse>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_ticker_price(&request.symbol).await {
        Ok((price, timestamp)) => {
            let response = MarketPriceResponse {
                symbol: request.symbol,
                price,
                timestamp,
            };
            Ok(Json(response))
        }
        Err(e) => {
            let error_response = ErrorResponse {
                error: format!("Failed to fetch market data: {}", e),
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

pub async fn get_book_ticker(
    State(service): State<BinanceMarketService>,
    Path(symbol): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_book_ticker(&symbol).await {
        Ok(ticker) => Ok(Json(ticker)),
        Err(e) => {
            let error_response = ErrorResponse {
                error: format!("Failed to fetch book ticker: {}", e),
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

pub async fn get_24h_stats(
    State(service): State<BinanceMarketService>,
    Path(symbol): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_24h_stats(&symbol).await {
        Ok(stats) => Ok(Json(stats)),
        Err(e) => {
            let error_response = ErrorResponse {
                error: format!("Failed to fetch 24h stats: {}", e),
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

pub async fn get_depth(
    State(service): State<BinanceMarketService>,
    Path(symbol): Path<String>,
    Query(params): Query<OrderBookRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_depth(&symbol, params.limit).await {
        Ok(depth) => Ok(Json(depth)),
        Err(e) => {
            let error_response = ErrorResponse {
                error: format!("Failed to fetch order book: {}", e),
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

// Update the get_klines handler to properly use the service

pub async fn get_klines(
    State(service): State<BinanceMarketService>,
    Path(symbol): Path<String>,
    Query(params): Query<KlineRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Default limit to 100 if not provided
    let limit = params.limit.unwrap_or(100);

    match service
        .get_historical_klines(&symbol, &params.interval, limit)
        .await
    {
        Ok(prices) => {
            // Convert the Vec<f64> to a more structured response
            let response = serde_json::json!({
                "symbol": symbol,
                "interval": params.interval,
                "prices": prices,
                "count": prices.len()
            });
            Ok(Json(response))
        }
        Err(e) => {
            let error_response = ErrorResponse {
                error: format!("Failed to fetch klines: {}", e),
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

// WebSocket handlers
pub async fn subscribe_websocket(
    State(service): State<BinanceWebSocketService>,
    Json(request): Json<WebSocketSubscriptionRequest>,
) -> Result<Json<WebSocketSubscriptionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate request
    if let Err(e) = request.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("{}", e),
            }),
        ));
    }

    let connection_id = match request.stream_type {
        StreamType::Ticker => service.subscribe_ticker(&request.symbol).await,
        StreamType::Kline => {
            let interval = request.interval.unwrap_or_else(|| "1m".to_string());
            service.subscribe_kline(&request.symbol, &interval).await
        }
        StreamType::Depth => service.subscribe_depth(&request.symbol).await,
    };

    match connection_id {
        Ok(id) => {
            let response = WebSocketSubscriptionResponse {
                connection_id: id,
                status: "subscribed".to_string(),
            };
            Ok(Json(response))
        }
        Err(e) => {
            let error_response = ErrorResponse {
                error: format!("Failed to subscribe: {}", e),
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

pub async fn unsubscribe_websocket(
    State(service): State<BinanceWebSocketService>,
    Path(connection_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match service.unsubscribe(&connection_id).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            let error_response = ErrorResponse {
                error: format!("Failed to unsubscribe: {}", e),
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

// Account handlers
pub async fn get_account(
    State(service): State<BinanceUserService>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_account().await {
        Ok(account) => Ok(Json(account)),
        Err(e) => {
            let error_response = ErrorResponse {
                error: format!("Failed to fetch account data: {}", e),
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

pub async fn get_balance(
    State(service): State<BinanceUserService>,
    Path(asset): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_balance(&asset).await {
        Ok(balance) => Ok(Json(balance)),
        Err(e) => {
            let error_response = ErrorResponse {
                error: format!("Failed to fetch balance: {}", e),
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

pub async fn get_open_orders(
    State(service): State<BinanceUserService>,
    Path(symbol): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_open_orders(&symbol).await {
        Ok(orders) => Ok(Json(orders)),
        Err(e) => {
            let error_response = ErrorResponse {
                error: format!("Failed to fetch open orders: {}", e),
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

pub async fn create_order(
    State(service): State<BinanceUserService>,
    Json(request): Json<OrderRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Validate request
    if let Err(e) = request.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("{}", e),
            }),
        ));
    }

    let result = match request.type_ {
        OrderType::Limit => {
            let price = request.price.ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "Price is required for limit orders".to_string(),
                    }),
                )
            })?;

            match request.side {
                OrderSide::Buy => {
                    service
                        .limit_buy(&request.symbol, request.quantity, price)
                        .await
                }
                OrderSide::Sell => {
                    service
                        .limit_sell(&request.symbol, request.quantity, price)
                        .await
                }
            }
        }
        OrderType::Market => match request.side {
            OrderSide::Buy => service.market_buy(&request.symbol, request.quantity).await,
            OrderSide::Sell => service.market_sell(&request.symbol, request.quantity).await,
        },
    };

    match result {
        Ok(order) => Ok(Json(order)),
        Err(e) => {
            let error_response = ErrorResponse {
                error: format!("Failed to create order: {}", e),
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

pub async fn cancel_order(
    State(service): State<BinanceUserService>,
    Path((symbol, order_id)): Path<(String, u64)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match service.cancel_order(&symbol, order_id).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            let error_response = ErrorResponse {
                error: format!("Failed to cancel order: {}", e),
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

pub async fn get_order_status(
    State(service): State<BinanceUserService>,
    Path((symbol, order_id)): Path<(String, u64)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_order_status(&symbol, order_id).await {
        Ok(status) => Ok(Json(status)),
        Err(e) => {
            let error_response = ErrorResponse {
                error: format!("Failed to get order status: {}", e),
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

pub async fn get_trade_history(
    State(service): State<BinanceUserService>,
    Path(symbol): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match service.get_trade_history(&symbol).await {
        Ok(trades) => Ok(Json(trades)),
        Err(e) => {
            let error_response = ErrorResponse {
                error: format!("Failed to get trade history: {}", e),
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}
