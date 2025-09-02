use axum::{extract::{Query, State}, routing::get, Json, Router};
use axum::response::Html;
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
        .route("/", get(index_handler))
        .route("/stats/clients", get(stats_clients_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index_handler() -> Html<&'static str> {
    // Simple Bootstrap-based page with a date range form and table
    Html(r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>rs-port-forward — Client Traffic</title>
    <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/css/bootstrap.min.css" rel="stylesheet" integrity="sha384-QWTKZyjpPEjISv5WaRU9OFeRpok6YctnYmDr5pNlyT2bRjXh0JMhjY6hW+ALEwIH" crossorigin="anonymous">
  </head>
  <body>
    <div class="container py-4">
      <h1 class="mb-4">Client traffic</h1>
      <form id="range-form" class="row gy-2 gx-3 align-items-end mb-4">
        <div class="col-auto">
          <label for="start" class="form-label">Start</label>
          <input type="datetime-local" id="start" class="form-control" required>
        </div>
        <div class="col-auto">
          <label for="end" class="form-label">End</label>
          <input type="datetime-local" id="end" class="form-control" required>
        </div>
        <div class="col-auto">
          <button class="btn btn-primary" type="submit">Load</button>
        </div>
      </form>

      <div id="alert" class="alert alert-danger d-none" role="alert"></div>

      <div class="table-responsive">
        <table class="table table-striped table-hover" id="stats-table">
          <thead>
            <tr>
              <th scope="col">#</th>
              <th scope="col">Client</th>
              <th scope="col">Bytes client→remote</th>
              <th scope="col">Bytes remote→client</th>
              <th scope="col">Total</th>
            </tr>
          </thead>
          <tbody></tbody>
        </table>
      </div>
    </div>

    <script>
      function toLocalInputValue(d) {
        const off = d.getTimezoneOffset();
        const local = new Date(d.getTime() - off * 60000);
        return local.toISOString().slice(0,16); // YYYY-MM-DDTHH:MM
      }

      function toISOStringFromInputValue(v) {
        // v is 'YYYY-MM-DDTHH:MM' in local time
        const d = new Date(v);
        return d.toISOString();
      }

      function fmtBytes(n) {
        let bytes = Number(n) || 0;
        const units = ['B','KB','MB','GB','TB','PB'];
        if (bytes < 1024) return `${bytes} B`;
        let i = Math.floor(Math.log(bytes) / Math.log(1024));
        i = Math.min(i, units.length - 1);
        const val = bytes / Math.pow(1024, i);
        const nf = new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 });
        return `${nf.format(val)} ${units[i]}`;
      }

      async function loadStats() {
        const alertBox = document.getElementById('alert');
        alertBox.classList.add('d-none');
        const startVal = document.getElementById('start').value;
        const endVal = document.getElementById('end').value;
        if (!startVal || !endVal) return;
        try {
          const startIso = toISOStringFromInputValue(startVal);
          const endIso = toISOStringFromInputValue(endVal);
          const res = await fetch(`/stats/clients?start=${encodeURIComponent(startIso)}&end=${encodeURIComponent(endIso)}`);
          if (!res.ok) throw new Error(`HTTP ${res.status}`);
          const data = await res.json();
          const tbody = document.querySelector('#stats-table tbody');
          tbody.innerHTML = '';
          data.forEach((row, idx) => {
            const total = (row.bytes_from_to || 0) + (row.bytes_to_from || 0);
            const tr = document.createElement('tr');
            tr.innerHTML = `
              <th scope="row">${idx+1}</th>
              <td>${row.client_addr ?? '<em>unknown</em>'}</td>
              <td>${fmtBytes(row.bytes_from_to)}</td>
              <td>${fmtBytes(row.bytes_to_from)}</td>
              <td>${fmtBytes(total)}</td>
            `;
            tbody.appendChild(tr);
          });
        } catch (e) {
          alertBox.textContent = 'Failed to load data: ' + e.message;
          alertBox.classList.remove('d-none');
        }
      }

      document.addEventListener('DOMContentLoaded', () => {
        const now = new Date();
        const weekAgo = new Date(now.getTime() - 7*24*60*60*1000);
        document.getElementById('start').value = toLocalInputValue(weekAgo);
        document.getElementById('end').value = toLocalInputValue(now);
        document.getElementById('range-form').addEventListener('submit', (e) => {
          e.preventDefault();
          loadStats();
        });
        loadStats();
      });
    </script>
  </body>
 </html>
"#)
}
