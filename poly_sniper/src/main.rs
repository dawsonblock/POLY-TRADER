use ethers::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;
use eyre::Result;
use ethers::contract::{Eip712, EthAbiType};

// --- CONFIGURATION ---

// Polymarket Safety Limits (Software HEAB)
const MAX_ORDER_SIZE_USDC: f64 = 500.0;
const MAX_INVENTORY_EXPOSURE_USDC: f64 = 5000.0;

// CTF Exchange Matching Engine Address on Polygon
const CTF_EXCHANGE_ADDRESS: &str = "0x4bFb41d5B3570DeFd03C39a9A4D8fE6bD8FCce67"; 

// --- EIP-712 POLYMARKET ORDER STRUCT ---
#[derive(Debug, Clone, Eip712, EthAbiType)]
#[eip712(
    name = "CTFExchange",
    version = "1",
    chain_id = 137,
    verifying_contract = "0x4bFb41d5B3570DeFd03C39a9A4D8fE6bD8FCce67"
)]
struct PolymarketOrder {
    salt: U256,
    maker: Address,
    signer: Address,
    taker: Address,
    tokenId: U256,
    makerAmount: U256,
    takerAmount: U256,
    expiration: U256,
    nonce: U256,
    feeRateBps: U256,
    side: u8,           // 0 = BUY, 1 = SELL
    signatureType: u8,  // 0 = EOA
}

#[derive(Deserialize, Debug)]
struct OracleSignal {
    timestamp: f64,
    asset: String,
    binance_price: f64,
    poly_fair_value: f64,
    action: String,
}

struct InventoryManager {
    current_exposure: f64,
}

impl InventoryManager {
    fn new() -> Self {
        Self { current_exposure: 0.0 }
    }

    fn check_and_update(&mut self, request_size: f64) -> bool {
        if request_size > MAX_ORDER_SIZE_USDC {
            println!("[HEAB DENIED] Order size ${} exceeds max ${}", request_size, MAX_ORDER_SIZE_USDC);
            return false;
        }
        if self.current_exposure + request_size > MAX_INVENTORY_EXPOSURE_USDC {
            println!("[HEAB DENIED] Max inventory exposure reached. Current: ${}", self.current_exposure);
            return false;
        }
        self.current_exposure += request_size;
        true
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    println!("[SNIPER] Booting Polymarket Execution Engine...");

    let private_key = std::env::var("POLY_PRIVATE_KEY").unwrap_or_else(|_| "0000000000000000000000000000000000000000000000000000000000000000".to_string());
    let rpc_url = std::env::var("POLY_RPC_WSS").expect("Missing POLY_RPC_WSS in .env");
    let zmq_addr = std::env::var("ZMQ_ADDR").unwrap_or_else(|_| "tcp://127.0.0.1:5555".to_string());

    // 1. Initialize Ethers Wallet & Provider
    let wallet = private_key.parse::<LocalWallet>()?.with_chain_id(137u64); // Polygon
    let _provider = Provider::<Ws>::connect(rpc_url.as_str()).await;
    // let client = SignerMiddleware::new(provider.unwrap(), wallet.clone());
    println!("[SNIPER] Zero-copy signing active for Polygon Wallet: {:?}", wallet.address());

    // 2. Initialize ZeroMQ
    let context = zmq::Context::new();
    let subscriber = context.socket(zmq::PULL).unwrap();
    subscriber
        .connect(zmq_addr.as_str())
        .expect("Failed connecting to ZMQ Oracle bridge");
    println!("[SNIPER] ZMQ IPC Bridge Connected -> Listening for sub-50us signals.");

    // 3. Initialize Software HEAB (Inventory Limits)
    let mut inventory = InventoryManager::new();

    let mut msg = zmq::Message::new();

    loop {
        // Block until we receive a signal from the Python Oracle
        subscriber.recv(&mut msg, 0).unwrap();
        let rcv_time = Instant::now();

        let payload = msg.as_str().unwrap();
        if let Ok(signal) = serde_json::from_str::<OracleSignal>(payload) {
            
            // Example Alpha Logic: "Buy 100 contracts if probability > 60%"
            if signal.poly_fair_value > 0.60 {
                
                let order_size = 500.0; // Hardcoded max clip
                
                if inventory.check_and_update(order_size) {
                    println!("[SNIPER] EXECUTING BUY! Target: {}, FairValue: {:.4}, Latency to parse: {:?}", 
                             signal.asset, signal.poly_fair_value, rcv_time.elapsed());
                    
                    // Craft EIP-712 Order
                    let target_token_id = U256::from_dec_str("12345678901234567890").unwrap(); // Contract Token ID
                    
                    let order = PolymarketOrder {
                        salt: U256::from(Instant::now().elapsed().as_nanos() as u64),
                        maker: wallet.address(),
                        signer: wallet.address(),
                        taker: Address::zero(), // Open order
                        tokenId: target_token_id,
                        makerAmount: U256::from(500_000_000), // 500 USDC (6 decimals)
                        takerAmount: U256::from(500_000_000), // Requesting 500 shares
                        expiration: U256::from(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + 60), // 60s expiry
                        nonce: U256::zero(), // Sequence nonce mapped from contract
                        feeRateBps: U256::zero(),
                        side: 0, // BUY
                        signatureType: 0, // EOA
                    };

                    // Sign Typed Data instantly in purely offline mode
                    let signature = wallet.sign_typed_data(&order).await?;
                    
                    println!("=> EIP-712 Signature Generated: {}", signature);
                    println!("=> Blast via REST payload: {{ order: {:?}, signature: {} }}", order, signature);
                    // In production, this JSON payload is written to a pre-warmed TCP socket hooked to Polymarket's /cancel-route or /order endpoint.
                }
            }
        }
    }
}
