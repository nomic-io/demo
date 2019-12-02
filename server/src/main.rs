use std::cmp::max;
use bitcoincore_rpc::{
    self as btc_rpc,
    json as btc_json,
    RpcApi
};

fn main() {
    let socket = ws::WebSocket::new(NullFactory).unwrap();
    let broadcast = socket.broadcaster();

    let btc = bitcoin_rpc(18443).unwrap();
    let mut btc_height = 0;
    std::thread::spawn(move || loop {
        let blocks = get_new_btc_blocks(&btc, btc_height).unwrap();
        blocks.last().map(|b| btc_height = b.height as u64);

        let message = format!("{:?}", blocks);
        
        broadcast.send(message).unwrap();
        std::thread::sleep_ms(1000);
    });

    socket.listen("localhost:8080").unwrap();
}

struct NullHandler;
impl ws::Handler for NullHandler {}

struct NullFactory;
impl ws::Factory for NullFactory {
    type Handler = NullHandler;
    
    fn connection_made(&mut self, conn: ws::Sender) -> NullHandler {
        NullHandler
    }
}

fn bitcoin_rpc(port: u16) -> btc_rpc::Result<btc_rpc::Client> {
    use btc_rpc::{Auth, Client};

    let user = std::env::var("BTC_RPC_USER")
        .expect("expected env var 'BTC_RPC_USER'");
    let pass = std::env::var("BTC_RPC_PASS")
        .expect("expected env var 'BTC_RPC_PASS'");
    let auth = Auth::UserPass(user, pass);
    Client::new(format!("http://localhost:{}", port), auth)
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
