use chrono::Utc;
use std::sync::Arc;
use tokio_rusqlite::Connection as AsyncConnection;

pub type SharedDb = Arc<AsyncConnection>;

#[derive(Clone, Debug)]
pub struct ConnectionRow {
    pub ts: String,
    pub name: String,
    pub log_name: String,
    pub local_port: u16,
    pub remote_address: String,
    pub remote_port: u16,
    pub client_addr: Option<String>,
    pub bytes_from_to: u64,
    pub bytes_to_from: u64,
}

pub async fn init_db(path: &str) -> anyhow::Result<SharedDb> {
    let conn = AsyncConnection::open(path).await?;
    // Create a simple table to store connection stats
    conn
        .call(|c: &mut rusqlite::Connection| -> tokio_rusqlite::Result<()> {
            c.execute(
                r#"
                CREATE TABLE IF NOT EXISTS connections (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    ts TEXT NOT NULL,
                    name TEXT,
                    log_name TEXT,
                    local_port INTEGER,
                    remote_address TEXT,
                    remote_port INTEGER,
                    client_addr TEXT,
                    bytes_from_to INTEGER,
                    bytes_to_from INTEGER
                );
                "#,
                [],
            )
            .map_err(tokio_rusqlite::Error::from)?;

            // Indexes to speed up lookups by remote_address and client_addr
            c.execute(
                "CREATE INDEX IF NOT EXISTS idx_connections_remote_address ON connections(remote_address)",
                [],
            )
            .map_err(tokio_rusqlite::Error::from)?;
            c.execute(
                "CREATE INDEX IF NOT EXISTS idx_connections_client_addr ON connections(client_addr)",
                [],
            )
            .map_err(tokio_rusqlite::Error::from)?;
            c.execute(
                "CREATE INDEX IF NOT EXISTS idx_connections_log_name ON connections(log_name)",
                [],
            )
            .map_err(tokio_rusqlite::Error::from)?;

            Ok(())
        })
        .await?;

    Ok(Arc::new(conn))
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_connection_row(
    db: &SharedDb,
    name: &str,
    local_port: u16,
    remote_address: &str,
    remote_port: u16,
    client_addr: Option<String>,
    bytes_from_to: u64,
    bytes_to_from: u64,
) -> anyhow::Result<()> {
    let ts = Utc::now().to_rfc3339();
    let name = name.to_string();
    let remote_address = remote_address.to_string();
    db
        .call(move |c: &mut rusqlite::Connection| -> tokio_rusqlite::Result<()> {
            let mut stmt = c
                .prepare(
                    "INSERT INTO connections (ts, name, local_port, remote_address, remote_port, client_addr, bytes_from_to, bytes_to_from)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                )
                .map_err(tokio_rusqlite::Error::from)?;
            stmt
                .execute(rusqlite::params![
                    ts,
                    name,
                    local_port as i64,
                    remote_address,
                    remote_port as i64,
                    client_addr,
                    bytes_from_to as i64,
                    bytes_to_from as i64
                ])
                .map(|_| ())
                .map_err(tokio_rusqlite::Error::from)
        })
    .await?;
    Ok(())
}

pub async fn insert_connection_rows(db: &SharedDb, rows: &[ConnectionRow]) -> anyhow::Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    // Clone values to move into blocking closure
    let rows_vec = rows.to_vec();
    db
        .call(move |c: &mut rusqlite::Connection| -> tokio_rusqlite::Result<()> {
            let tx = c.transaction().map_err(tokio_rusqlite::Error::from)?;
            {
                let mut stmt = tx
                    .prepare(
                        "INSERT INTO connections (ts, name, log_name, local_port, remote_address, remote_port, client_addr, bytes_from_to, bytes_to_from)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    )
                    .map_err(tokio_rusqlite::Error::from)?;
                for r in rows_vec.iter() {
                    stmt
                        .execute(rusqlite::params![
                            r.ts,
                            r.name,
                            r.log_name,
                            r.local_port as i64,
                            r.remote_address,
                            r.remote_port as i64,
                            r.client_addr,
                            r.bytes_from_to as i64,
                            r.bytes_to_from as i64
                        ])
                        .map_err(tokio_rusqlite::Error::from)?;
                }
            }
            tx.commit().map_err(tokio_rusqlite::Error::from)?;
            Ok(())
        })
        .await?;
    Ok(())
}
