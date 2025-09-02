use axum::{extract::{Query, State}, routing::get, Json, Router};
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use std::net::SocketAddr;

use crate::db::{query_traffic_by_client, ClientTraffic, SharedDb};

#[derive(Clone)]
pub struct AppState {
    pub db: Option<SharedDb>,
}

#[derive(Deserialize)]
pub struct StatsQuery {
    pub start: String,
    pub end: String,
}

fn parse_time(s: &str) -> Result<DateTime<Utc>, String> {
    if let Ok(secs) = s.parse::<i64>() {
        return Utc
            .timestamp_opt(secs, 0)
            .single()
            .ok_or_else(|| "invalid unix timestamp".to_string());
    }
    s.parse::<DateTime<Utc>>()
        .map_err(|_| "invalid time format, use RFC3339 or unix seconds".to_string())
}

async fn stats_clients_handler(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> Result<Json<Vec<ClientTraffic>>, (axum::http::StatusCode, String)> {
    let start = parse_time(&q.start).map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;
    let end = parse_time(&q.end).map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;

    if let Some(db) = state.db {
        let rows = query_traffic_by_client(&db, start, end)
            .await
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Ok(Json(rows))
    } else {
        Err((
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "database is not configured".to_string(),
        ))
    }
}

pub async fn run_http(addr: &str, state: AppState) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/stats/clients", get(stats_clients_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

