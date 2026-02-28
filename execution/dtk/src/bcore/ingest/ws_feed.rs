/*
 * BOREAL INGEST: RUST WEBSOCKET FEED
 * Replaces Python poly_oracle.py entirely.
 * Goal: Zero-copy tick parsing with NIC-level timestamps.
 */

use serde::Deserialize;
use tokio_tungstenite::connect_async;
use futures_util::StreamExt;
use crossbeam::queue::ArrayQueue;
use std::sync::Arc;

use crate::bcore::features::fixed_point::Fixed;
use crate::bcore::feed::tick::Tick;

// Binance miniTicker stream format
#[derive(Deserialize)]
struct BinanceTicker {
    #[serde(rename = "c")]
    close: String,  // Last price as string (Binance sends strings for precision)
    #[serde(rename = "v")]
    volume: String,
}

/// Spawn the ingest task. Parses frames off the WebSocket, converts to
/// canonical `Tick` structs, and pushes into the shared lock-free ring.
pub async fn run_ingest(
    symbol: &str,
    ring: Arc<ArrayQueue<Tick>>,
    mut seq_counter: u64,
) {
    let url = format!(
        "wss://stream.binance.com:9443/ws/{}@miniTicker",
        symbol.to_lowercase()
    );

    loop {
        match connect_async(&url).await {
            Err(e) => {
                eprintln!("[INGEST] WebSocket connect failed: {e}. Retrying in 1s...");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
            Ok((mut ws, _)) => {
                eprintln!("[INGEST] Connected to {url}");

                while let Some(msg) = ws.next().await {
                    match msg {
                        Ok(tokio_tungstenite::tungstenite::Message::Text(raw)) => {
                            // Capture monotonic timestamp immediately upon frame receipt
                            // On Linux production: replace with SO_TIMESTAMPING ioctl
                            let ts_mono_ns = {
                                use std::time::{SystemTime, UNIX_EPOCH};
                                SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_nanos() as u64
                            };

                            if let Ok(ticker) = serde_json::from_str::<BinanceTicker>(&raw) {
                                let price_f: f64 = ticker.close.parse().unwrap_or(0.0);
                                let size_f: f64  = ticker.volume.parse().unwrap_or(0.0);

                                // Convert to canonical fixed-point representation
                                // ALL downstream logic is Q32.32 — no floats cross this boundary
                                let tick = Tick {
                                    seq: seq_counter,
                                    source_id: 1, // Binance = source 1
                                    ts_mono_ns,
                                    raw_hash: [0u8; 32], // Populated by ledger capture
                                    price: Fixed::from_f64(price_f),
                                    size:  Fixed::from_f64(size_f),
                                };

                                // Non-blocking push. If ring is full (backpressure),
                                // we DROP the tick — never block the ingest thread.
                                if ring.push(tick).is_err() {
                                    eprintln!("[INGEST][WARN] Ring full — tick dropped (backpressure)");
                                }

                                seq_counter += 1;
                            }
                        }
                        Err(e) => {
                            eprintln!("[INGEST] WebSocket error: {e}. Reconnecting...");
                            break; // Reconnect outer loop
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
