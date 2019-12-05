use std::cmp::{min, max};
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
    Server::new("localhost:18443", "kep.io:26657").unwrap()
        .listen("localhost:8080").unwrap()
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

struct Server {
    websocket: Option<ws::WebSocket<NullFactory>>,
    broadcaster: ws::Sender,
    bitcoin_rpc: btc_rpc::Client,
    tendermint_rpc: tm_rpc::Client,
    bitcoin_blocks: Vec<btc_json::GetBlockResult>,
    tendermint_blocks: Vec<tendermint::block::Block>
}

impl Server {
    pub fn new(btc_addr: &str, tm_addr: &str) -> Result<Self> {
        let websocket = ws::WebSocket::new(NullFactory)?;
        let broadcaster = websocket.broadcaster();

        let bitcoin_rpc = connect_btc_rpc(btc_addr)?;
        let tendermint_rpc = connect_tm_rpc(tm_addr)?;

        print!("fetching initial block state...");
        let tendermint_blocks = get_initial_tm_blocks(&tendermint_rpc)?;
        println!(" done");

        Ok(Server {
            websocket: Some(websocket),
            broadcaster,
            bitcoin_rpc,
            tendermint_rpc,
            bitcoin_blocks: vec![],
            tendermint_blocks
        })
    }

    pub fn listen(mut self, listen_addr: &str) -> Result<()> {
        let websocket = self.websocket.take().unwrap();

        std::thread::spawn(move || loop {
            let btc_blocks = self.get_new_btc_blocks().unwrap();

            if !btc_blocks.is_empty() {
                let message = serde_json::to_string(&btc_blocks).unwrap();
                self.broadcast(format!("{}\n", message)).unwrap();
            }

            std::thread::sleep_ms(1000);
        });

        websocket.listen(listen_addr)?;

        bail!("server stopped")
    }

    fn broadcast(&self, msg: String) -> Result<()> {
        self.broadcaster.send(msg)?;
        Ok(())
    }

    fn btc_height(&self) -> u64 {
        match self.bitcoin_blocks.last() {
            None => 0,
            Some(block) => block.height as u64
        }
    }

    fn get_new_btc_blocks(
        &mut self
    ) -> btc_rpc::Result<Vec<btc_json::GetBlockResult>> {
        let height = self.bitcoin_rpc.get_block_count()?;

        let mut blocks = vec![];
        let start_height = max(
            self.btc_height(),
            max(height, 5) - 5
        ) + 1;

        for h in start_height..=height {
            let hash = self.bitcoin_rpc.get_block_hash(h)?;
            let block = self.bitcoin_rpc.get_block_info(&hash)?;
            blocks.push(block.clone());
            self.bitcoin_blocks.push(block);
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
) -> Result<Vec<tendermint::block::Block>> {
    let height = rpc.status()?.sync_info.latest_block_height;

    let mut blocks: Vec<tendermint::block::Block> = vec![];

    for i in 0..500 {
        let block = rpc.block(height.value() - i)?.block;

        let before_populated = match blocks.last() {
            None => true,
            Some(prev) => prev.header.height == block.header.height
        };

        if block.header.num_txs == 0 && !before_populated {
            continue;
        }

        blocks.push(block);
    }

    Ok(blocks)
}
