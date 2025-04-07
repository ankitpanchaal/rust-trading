use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use validator::Validate;

use crate::error::AppError;
use super::{
    market_service::BinanceMarketService, model::*, user_service::BinanceUserService,
    websocket_service::BinanceWebSocketService,
};

// Market handlers
pub async fn get_price(
    State(service): State<BinanceMarketService>,
    Path(symbol): Path<String>,
) -> Result<Json<MarketPriceResponse>, AppError> {
    let (price, timestamp) = service
        .get_ticker_price(&symbol)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch market data: {}", e)))?;
    
    let response = MarketPriceResponse {
        symbol,
        price,
        timestamp,
    };
    Ok(Json(response))
}

pub async fn get_price_post(
    State(service): State<BinanceMarketService>,
    Json(request): Json<MarketPriceRequest>,
) -> Result<Json<MarketPriceResponse>, AppError> {
    let (price, timestamp) = service
        .get_ticker_price(&request.symbol)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch market data: {}", e)))?;
    
    let response = MarketPriceResponse {
        symbol: request.symbol,
        price,
        timestamp,
    };
    Ok(Json(response))
}

pub async fn get_book_ticker(
    State(service): State<BinanceMarketService>,
    Path(symbol): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ticker = service
        .get_book_ticker(&symbol)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch book ticker: {}", e)))?;
    
    Ok(Json(ticker))
}

pub async fn get_24h_stats(
    State(service): State<BinanceMarketService>,
    Path(symbol): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let stats = service
        .get_24h_stats(&symbol)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch 24h stats: {}", e)))?;
    
    Ok(Json(stats))
}

pub async fn get_depth(
    State(service): State<BinanceMarketService>,
    Path(symbol): Path<String>,
    Query(params): Query<OrderBookRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let depth = service
        .get_depth(&symbol, params.limit)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch order book: {}", e)))?;
    
    Ok(Json(depth))
}

pub async fn get_klines(
    State(service): State<BinanceMarketService>,
    Path(symbol): Path<String>,
    Query(params): Query<KlineRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Default limit to 100 if not provided
    let limit = params.limit.unwrap_or(100);

    let prices = service
        .get_historical_klines(&symbol, &params.interval, limit)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch klines: {}", e)))?;
    
    // Convert the Vec<f64> to a more structured response
    let response = serde_json::json!({
        "symbol": symbol,
        "interval": params.interval,
        "prices": prices,
        "count": prices.len()
    });
    Ok(Json(response))
}

// WebSocket handlers
pub async fn subscribe_websocket(
    State(service): State<BinanceWebSocketService>,
    Json(request): Json<WebSocketSubscriptionRequest>,
) -> Result<Json<WebSocketSubscriptionResponse>, AppError> {
    // Validate request
    if let Err(e) = request.validate() {
        return Err(AppError::ValidationError(format!("{}", e)));
    }

    let connection_id = match request.stream_type {
        StreamType::Ticker => service.subscribe_ticker(&request.symbol).await,
        StreamType::Kline => {
            let interval = request.interval.unwrap_or_else(|| "1m".to_string());
            service.subscribe_kline(&request.symbol, &interval).await
        }
        StreamType::Depth => service.subscribe_depth(&request.symbol).await,
    }.map_err(|e| AppError::InternalError(format!("Failed to subscribe: {}", e)))?;

    let response = WebSocketSubscriptionResponse {
        connection_id,
        status: "subscribed".to_string(),
    };
    Ok(Json(response))
}

pub async fn unsubscribe_websocket(
    State(service): State<BinanceWebSocketService>,
    Path(connection_id): Path<String>,
) -> Result<StatusCode, AppError> {
    service
        .unsubscribe(&connection_id)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to unsubscribe: {}", e)))?;
    
    Ok(StatusCode::NO_CONTENT)
}

// Account handlers
pub async fn get_account(
    State(service): State<BinanceUserService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let account = service
        .get_account()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch account data: {}", e)))?;
    
    Ok(Json(account))
}

pub async fn get_balance(
    State(service): State<BinanceUserService>,
    Path(asset): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let balance = service
        .get_balance(&asset)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch balance: {}", e)))?;
    
    Ok(Json(balance))
}

pub async fn get_open_orders(
    State(service): State<BinanceUserService>,
    Path(symbol): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let orders = service
        .get_open_orders(&symbol)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch open orders: {}", e)))?;
    
    Ok(Json(orders))
}

pub async fn create_order(
    State(service): State<BinanceUserService>,
    Json(request): Json<OrderRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Validate request
    if let Err(e) = request.validate() {
        return Err(AppError::ValidationError(format!("{}", e)));
    }

    let result = match request.type_ {
        OrderType::Limit => {
            let price = request.price.ok_or_else(|| {
                AppError::ValidationError("Price is required for limit orders".to_string())
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

    let order = result
        .map_err(|e| AppError::InternalError(format!("Failed to create order: {}", e)))?;
    
    Ok(Json(order))
}

pub async fn cancel_order(
    State(service): State<BinanceUserService>,
    Path((symbol, order_id)): Path<(String, u64)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let response = service
        .cancel_order(&symbol, order_id)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to cancel order: {}", e)))?;
    
    Ok(Json(response))
}

pub async fn get_order_status(
    State(service): State<BinanceUserService>,
    Path((symbol, order_id)): Path<(String, u64)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let status = service
        .get_order_status(&symbol, order_id)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to get order status: {}", e)))?;
    
    Ok(Json(status))
}

pub async fn get_trade_history(
    State(service): State<BinanceUserService>,
    Path(symbol): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let trades = service
        .get_trade_history(&symbol)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to get trade history: {}", e)))?;
    
    Ok(Json(trades))
}