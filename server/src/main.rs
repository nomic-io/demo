use std::cmp::max;
use bitcoincore_rpc::{
    self as btc_rpc,
    json as btc_json,
    RpcApi
};
use tendermint::rpc as tm_rpc;
use serde::{Deserialize, Serialize};

fn main() {
    let websocket = ws::WebSocket::new(NullFactory).unwrap();
    let broadcast = websocket.broadcaster();

    let btc = bitcoin_rpc("localhost:18443").unwrap();
    let mut btc_height = 0;

    let tm = tendermint_rpc("kep.io:26657").unwrap();
    let mut tm_height = 0;

    std::thread::spawn(move || loop {
        let btc_blocks = get_new_btc_blocks(&btc, btc_height).unwrap();
        btc_blocks.last().map(|b| btc_height = b.height as u64);

        let message = serde_json::to_string(&btc_blocks).unwrap();
        broadcast.send(message).unwrap();

        let tm_blocks = get_new_tm_blocks(&tm, tm_height).unwrap();
        tm_blocks.last().map(|b| tm_height = b.height.value());

        let message = serde_json::to_string(&tm_blocks).unwrap();
        broadcast.send(message).unwrap();

        std::thread::sleep_ms(500);
    });

    websocket.listen("localhost:8080").unwrap();
}

struct NullHandler;
impl ws::Handler for NullHandler {}

struct NullFactory;
impl ws::Factory for NullFactory {
    type Handler = NullHandler;
    
    fn connection_made(&mut self, _conn: ws::Sender) -> NullHandler {
        NullHandler
    }
}

fn bitcoin_rpc(address: &str) -> btc_rpc::Result<btc_rpc::Client> {
    use btc_rpc::{Auth, Client};

    let user = std::env::var("BTC_RPC_USER")
        .expect("expected env var 'BTC_RPC_USER'");
    let pass = std::env::var("BTC_RPC_PASS")
        .expect("expected env var 'BTC_RPC_PASS'");
    let auth = Auth::UserPass(user, pass);
    Client::new(format!("http://{}", address), auth)
}

fn get_new_btc_blocks(
    rpc: &btc_rpc::Client,
    current_height: u64
) -> btc_rpc::Result<Vec<btc_json::GetBlockResult>> {
    let height = rpc.get_block_count()?;

    let mut blocks = vec![];
    let start_height = max(
        current_height,
        max(height, 5) - 5
    ) + 1;

    for h in start_height..=height {
        let hash = rpc.get_block_hash(h)?;
        let block = rpc.get_block_info(&hash)?;
        blocks.push(block);
    }

    Ok(blocks)
}

fn tendermint_rpc(
    address: &str
) -> Result<tm_rpc::Client, tm_rpc::Error> {
    let address = address.parse().expect("invalid address");
    tm_rpc::Client::new(&address)
}

fn get_new_tm_blocks(
    rpc: &tm_rpc::Client,
    current_height: u64
) -> Result<Vec<tendermint::block::Header>, tm_rpc::Error> {
    let height = rpc.status()?.sync_info.latest_block_height.value();

    let mut blocks = vec![];
    let start_height = max(
        current_height,
        max(height, 5) - 5
    ) + 1;

    for h in start_height..=height {
        let res = rpc.block(h)?;
        blocks.push(res.block.header);
    }

    Ok(blocks)
}
