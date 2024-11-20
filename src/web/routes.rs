use std::collections::HashMap;
use std::sync::Arc;

use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use async_graphql_rocket::{GraphQLRequest, GraphQLResponse};
use log::{info, warn};
use rocket::request::FromParam;
use rocket::response::content;
use rocket::serde::json::Json;
use rocket::{get, routes, Route, State};
use rocket_okapi::swagger_ui::SwaggerUIConfig;
use rocket_okapi::{openapi, openapi_get_routes, JsonSchema};
use serde::{Deserialize, Serialize};

use crate::indexer::spot_order::{OrderType, SpotOrder};
use crate::storage::candles::CandleStore;
use crate::storage::order_book::OrderBook;

use super::graphql::Query;

#[derive(Serialize, JsonSchema)]
pub struct OrdersResponse {
    pub orders: Vec<SpotOrder>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub enum Indexer {
    Envio,
    Subsquid,
    Pangea,
}

impl Indexer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Indexer::Envio => "envio",
            Indexer::Subsquid => "subsquid",
            Indexer::Pangea => "superchain",
        }
    }

    pub fn all() -> Vec<Indexer> {
        vec![Indexer::Envio, Indexer::Subsquid, Indexer::Pangea]
    }
}

impl<'r> FromParam<'r> for Indexer {
    type Error = &'r str;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        match param {
            "Envio" => Ok(Indexer::Envio),
            "Subsquid" => Ok(Indexer::Subsquid),
            "Pangea" => Ok(Indexer::Pangea),
            _ => Err(param),
        }
    }
}

#[derive(serde::Serialize, JsonSchema)]
pub struct AdvancedChartResponse {
    s: String,            // Статус ("ok" или "no_data")
    t: Vec<u64>,          // Временные метки
    o: Vec<f64>,          // Открытие
    h: Vec<f64>,          // Максимум
    l: Vec<f64>,          // Минимум
    c: Vec<f64>,          // Закрытие
    v: Vec<f64>,          // Объём
}

#[openapi]
#[get("/timestamps")]
fn get_timestamps(candle_store: &State<Arc<CandleStore>>) -> Json<Option<(i64, i64)>> {
    let min_max = candle_store.get_min_max_timestamps();
    Json(min_max)
}



#[openapi]
#[get("/config")]
fn get_config() -> Json<HashMap<&'static str, &'static str>> {
    let mut config = HashMap::new();
    config.insert("chartsStorageUrl", "https://saveload.tradingview.com");
    config.insert("chartsStorageApiVersion", "1.1");
    config.insert("clientId", "tradingview.com");
    config.insert("userId", "public_user_id");
    Json(config)
}


#[openapi]
#[get("/time")]
fn get_time() -> Json<u64> {
    let timestamp = chrono::Utc::now().timestamp() as u64;
    Json(timestamp)
}

#[derive(Serialize, JsonSchema)]
pub struct SymbolInfo {
    pub symbol: String,
    pub name: String,
    pub description: String,
    pub type_: String,
    pub exchange: String,
    pub timezone: String,
}

#[openapi]
#[get("/symbols")]
fn get_symbols() -> Json<Vec<SymbolInfo>> {
    let symbols = vec![
        SymbolInfo {
            symbol: "BTC/USD".to_string(),
            name: "Bitcoin / US Dollar".to_string(),
            description: "BTC to USD".to_string(),
            type_: "crypto".to_string(),
            exchange: "CryptoExchange".to_string(),
            timezone: "Etc/UTC".to_string(),
        },
        SymbolInfo {
            symbol: "ETH/USD".to_string(),
            name: "Ethereum / US Dollar".to_string(),
            description: "ETH to USD".to_string(),
            type_: "crypto".to_string(),
            exchange: "CryptoExchange".to_string(),
            timezone: "Etc/UTC".to_string(),
        },
    ];
    Json(symbols)
}

#[openapi]
#[get("/history?<symbol>&<resolution>&<from>&<to>")]
fn get_history(
    candle_store: &State<Arc<CandleStore>>,
    symbol: String,
    resolution: u64,
    from: u64,
    to: u64,
) -> Json<AdvancedChartResponse> {
    let candles = candle_store.get_candles_in_time_range(&symbol, resolution, from, to);
    if candles.is_empty() {
        Json(AdvancedChartResponse {
            s: "no_data".to_string(),
            t: vec![],
            o: vec![],
            h: vec![],
            l: vec![],
            c: vec![],
            v: vec![],
        })
    } else {
        let t: Vec<u64> = candles.iter().map(|c| c.timestamp.timestamp() as u64).collect();
        let o: Vec<f64> = candles.iter().map(|c| c.open).collect();
        let h: Vec<f64> = candles.iter().map(|c| c.high).collect();
        let l: Vec<f64> = candles.iter().map(|c| c.low).collect();
        let c: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let v: Vec<f64> = candles.iter().map(|c| c.volume).collect();

        Json(AdvancedChartResponse {
            s: "ok".to_string(),
            t,
            o,
            h,
            l,
            c,
            v,
        })
    }
}


#[openapi]
#[get("/candles?<symbol>&<interval>&<from>&<to>")]
pub fn get_candles(
    candle_store: &State<Arc<CandleStore>>,
    symbol: String,
    interval: u64,
    from: u64,
    to: u64,
) -> Json<AdvancedChartResponse> {
    let candles = candle_store
        .get_candles_in_time_range(&symbol, interval, from, to);
    //info!("=====================");
    //info!("candle_store: {:?}", candle_store);
    //info!("=====================");

    if candles.is_empty() {
        Json(AdvancedChartResponse {
            s: "no_data".to_string(),
            t: vec![],
            o: vec![],
            h: vec![],
            l: vec![],
            c: vec![],
            v: vec![],
        })
    } else {
        let t: Vec<u64> = candles.iter().map(|c| c.timestamp.timestamp() as u64).collect();
        let o: Vec<f64> = candles.iter().map(|c| c.open).collect();
        let h: Vec<f64> = candles.iter().map(|c| c.high).collect();
        let l: Vec<f64> = candles.iter().map(|c| c.low).collect();
        let c: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let v: Vec<f64> = candles.iter().map(|c| c.volume).collect();

        Json(AdvancedChartResponse {
            s: "ok".to_string(),
            t,
            o,
            h,
            l,
            c,
            v,
        })
    }
}


#[rocket::post("/graphql", data = "<request>")]
pub async fn graphql_handler(
    schema: &State<Schema<Query, EmptyMutation, EmptySubscription>>,
    request: GraphQLRequest,
) -> GraphQLResponse {
    request.execute(&**schema).await 
}

#[rocket::get("/graphql/playground")]
pub fn graphql_playground() -> content::RawHtml<String> {
    warn!("======GQPLGRND========");
    let gqlpgc = GraphQLPlaygroundConfig::new("/api/graphql");

    content::RawHtml(playground_source(gqlpgc))
}

pub fn get_routes() -> Vec<Route> {
    openapi_get_routes![
        get_config,
        get_time,
        get_symbols,
        get_candles,
        get_timestamps,
        get_history
    ]
}

pub fn get_graphql_routes() -> Vec<Route> {
    routes![graphql_handler, graphql_playground]
}

pub fn get_docs() -> SwaggerUIConfig {
    SwaggerUIConfig {
        url: "/openapi.json".to_string(),
        ..Default::default()
    }
}
