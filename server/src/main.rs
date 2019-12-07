use std::cmp::{min, max};
use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Mutex};
use bitcoincore_rpc::{
    self as btc_rpc,
    json as btc_json,
    RpcApi
};
use tendermint::rpc as tm_rpc;
use serde::{Deserialize, Serialize};
use failure::{bail, Error};

type Result<T> = std::result::Result<T, Error>;

fn main() {
    let server = Server::new("localhost:18332", "kep.io:26657").unwrap();

    let mut settings: ws::Settings = Default::default();
    settings.out_buffer_capacity = 1024 * 128;
    settings.max_connections = 512;

    let factory = WSFactory(
        server.bitcoin_blocks.clone(),
        server.tendermint_blocks.clone()
    );

    let websocket = ws::Builder::new()
        .with_settings(settings)
        .build(factory)
        .unwrap();
    let broadcaster = websocket.broadcaster();

    server.start(broadcaster);
    
    websocket.listen("0.0.0.0:8080").unwrap();
}

struct NullHandler;
impl ws::Handler for NullHandler {}

struct WSFactory (
    Arc<Mutex<VecDeque<btc_json::GetBlockResult>>>,
    Arc<Mutex<VecDeque<tendermint::block::Block>>>
);
impl ws::Factory for WSFactory {
    type Handler = NullHandler;
    
    fn connection_made(&mut self, conn: ws::Sender) -> NullHandler {
        eprintln!("incoming connection");

        let update = Update {
            bitcoin: self.0.lock().unwrap().clone().into_iter().collect(),
            tendermint: self.1.lock().unwrap().clone().into_iter().collect()
        };
        let message = serde_json::to_string(&update).unwrap();
        conn.send(format!("{}\n", message)).unwrap();

        NullHandler
    }
}

#[derive(Serialize)]
struct Update {
    bitcoin: Vec<btc_json::GetBlockResult>,
    tendermint: Vec<tendermint::block::Block>
}

struct Server {
    bitcoin_rpc: btc_rpc::Client,
    tendermint_rpc: tm_rpc::Client,
    bitcoin_blocks: Arc<Mutex<VecDeque<btc_json::GetBlockResult>>>,
    tendermint_blocks: Arc<Mutex<VecDeque<tendermint::block::Block>>>
}

impl Server {
    pub fn new(btc_addr: &str, tm_addr: &str) -> Result<Self> {
        let bitcoin_rpc = connect_btc_rpc(btc_addr)?;
        let tendermint_rpc = connect_tm_rpc(tm_addr)?;

        eprint!("fetching initial block state...");
        std::io::stderr().flush()?;
        let tendermint_blocks = get_initial_tm_blocks(&tendermint_rpc)?;
        eprintln!(" done");

        Ok(Server {
            bitcoin_rpc,
            tendermint_rpc,
            bitcoin_blocks: Default::default(),
            tendermint_blocks: Arc::new(Mutex::new(tendermint_blocks))
        })
    }

    pub fn start(mut self, broadcaster: ws::Sender) {
        std::thread::spawn(move || loop {
            let btc_blocks = self.get_new_btc_blocks().unwrap();
            let tm_blocks = self.get_new_tm_blocks().unwrap();

            if !btc_blocks.is_empty() || !tm_blocks.is_empty() {
                let update = Update {
                    bitcoin: btc_blocks,
                    tendermint: tm_blocks
                };
                let message = serde_json::to_string(&update).unwrap();
                broadcaster.send(format!("{}\n", message)).unwrap();
            }

            std::thread::sleep_ms(1000);
        });
    }

    fn btc_height(&self) -> u64 {
        let bitcoin_blocks = self.bitcoin_blocks.lock().unwrap();
        match bitcoin_blocks.back() {
            None => 0,
            Some(block) => block.height as u64
        }
    }

    fn get_new_btc_blocks(
        &mut self
    ) -> Result<Vec<btc_json::GetBlockResult>> {
        let height = self.bitcoin_rpc.get_block_count()?;

        let mut blocks = vec![];
        let start_height = max(
            self.btc_height(),
            max(height, 5) - 5
        ) + 1;

        let mut bitcoin_blocks = self.bitcoin_blocks.lock().unwrap();

        for h in start_height..=height {
            let hash = self.bitcoin_rpc.get_block_hash(h)?;
            let block = self.bitcoin_rpc.get_block_info(&hash)?;
            blocks.push(block.clone());

            bitcoin_blocks.push_back(block);
            while bitcoin_blocks.len() > 3 {
                bitcoin_blocks.pop_front();
            }
        }

        Ok(blocks)
    }

    fn tm_height(&self) -> u64 {
        let tendermint_blocks = self.tendermint_blocks.lock().unwrap();
        match tendermint_blocks.back() {
            None => 0,
            Some(block) => block.header.height.value()
        }
    }

    fn get_new_tm_blocks(&mut self) -> Result<Vec<tendermint::block::Block>> {
        let height = self.tendermint_rpc.status()?
            .sync_info.latest_block_height.value();
        
        let mut blocks = vec![];
        let start_height = max(
            self.tm_height(),
            max(height, 5) - 5
        ) + 1;

        let mut tendermint_blocks = self.tendermint_blocks.lock().unwrap();

        for h in start_height..=height {
            let res = self.tendermint_rpc.block(h)?;

            let populated = has_header_tx(&res.block);
            let after_populated = has_header_tx(tendermint_blocks.back().unwrap());

            // replace last
            if !populated && !after_populated {
                if !blocks.is_empty() {
                    blocks.pop();
                }
                tendermint_blocks.pop_back();
            }

            blocks.push(res.block.clone());
            tendermint_blocks.push_back(res.block);
            while tendermint_blocks.len() > 6 {
                tendermint_blocks.pop_front();
            }
        }

        Ok(blocks)
    }
}

fn connect_btc_rpc(address: &str) -> btc_rpc::Result<btc_rpc::Client> {
    use btc_rpc::{Auth, Client};

    let user = std::env::var("BTC_RPC_USER")
        .expect("expected env var 'BTC_RPC_USER'");
    let pass = std::env::var("BTC_RPC_PASS")
        .expect("expected env var 'BTC_RPC_PASS'");
    let auth = Auth::UserPass(user, pass);
    Client::new(format!("http://{}", address), auth)
}

fn connect_tm_rpc(address: &str) -> Result<tm_rpc::Client> {
    let address = address.parse().expect("invalid address");
    Ok(tm_rpc::Client::new(&address)?)
}

fn get_initial_tm_blocks(
    rpc: &tm_rpc::Client
) -> Result<VecDeque<tendermint::block::Block>> {
    let height = rpc.status()?.sync_info.latest_block_height;

    let mut blocks: VecDeque<tendermint::block::Block> = Default::default();

    for i in 0..500 {
        if i == height.value() {
            break;
        }

        let block = rpc.block(height.value() - i)?.block;

        let before_populated = match blocks.back() {
            None => true,
            Some(prev) => prev.header.height.value() + 1 == block.header.height.value()
        };

        if has_header_tx(&block) || before_populated {
            blocks.push_back(block);
        }
    }

    Ok(blocks)
}

fn has_header_tx(block: &tendermint::block::Block) -> bool {
    for tx in block.data.iter() {
        if &tx.as_bytes()[2..8] == b"Header" {
            return true;
        }
    }

    false
}
