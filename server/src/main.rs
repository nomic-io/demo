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
use sha2::{Digest, Sha256};
use bitcoin::blockdata::block::BlockHeader;
use bitcoin::util::hash::BitcoinHash;

type Result<T> = std::result::Result<T, Error>;

fn main() {
    let server = Server::new("localhost:18332", "localhost:26657").unwrap();

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
    Arc<Mutex<VecDeque<BtcBlock>>>,
    Arc<Mutex<VecDeque<TmBlock>>>
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

#[derive(Serialize, Clone)]
enum TmTx {
    HeaderRelay(Vec<String>),
    WorkProof {
        value: u64,
        pubkey: String
    }
}

impl From<tendermint::abci::transaction::Transaction> for TmTx {
    fn from(tx: tendermint::abci::transaction::Transaction) -> Self {
        let bytes = tx.into_vec();
        let json: serde_json::Value = serde_json::from_slice(bytes.as_slice()).unwrap();
        if let serde_json::Value::Object(object) = &json["Header"] {
            let mut headers = vec![];
            for header_value in object["block_headers"].as_array().unwrap() {
                let header_json = serde_json::to_string(header_value).unwrap();
                let header: BlockHeader = serde_json::from_str(&header_json).unwrap();
                let mut hash_bytes = header.bitcoin_hash().as_ref().to_vec();
                hash_bytes.reverse();
                let mut hash = String::new();
                for byte in hash_bytes {
                    hash.extend(format!("{:02x}", byte).chars());
                }

                headers.push(hash)
            }

            TmTx::HeaderRelay(headers)
        } else if let serde_json::Value::Object(object) = &json["WorkProof"] {
            let pubkey_arr = object["public_key"].as_array().unwrap();
            let pubkey_bytes: Vec<u8> = pubkey_arr
                .into_iter()
                .map(|byte_value| byte_value.as_u64().unwrap() as u8)
                .collect();

            let nonce = object["nonce"].as_u64().unwrap();
            let nonce_bytes = nonce.to_be_bytes();

            let mut hasher = Sha256::new();
            hasher.input(pubkey_bytes);
            hasher.input(&nonce_bytes);
            let hash = hasher.result().to_vec();

            let mut leading_zeros = 0;
            for byte in hash {
                leading_zeros += byte.leading_zeros();
                if byte.leading_zeros() != 8 { break }
            }

            let mut pubkey = String::new();
            for byte_value in pubkey_arr {
                let byte = byte_value.as_u64().unwrap() as u8;
                pubkey.extend(format!("{:x}", byte).chars());
            }

            TmTx::WorkProof {
                value: 1 << leading_zeros,
                pubkey
            }
        } else {
            println!("{:?}", json);
            panic!("unknown transaction type")
        }
    }
}

#[derive(Serialize, Clone)]
struct TmBlock {
    height: u64,
    hash: tendermint::hash::Hash,
    time: tendermint::time::Time,
    txs: Vec<TmTx>
}

impl From<tm_rpc::endpoint::block::Response> for TmBlock {
    fn from(block: tm_rpc::endpoint::block::Response) -> Self {
        let txs = block.block.data.into_vec().into_iter()
            .map(|tx| tx.into())
            .collect();

        TmBlock {
            height: block.block.header.height.value(),
            hash: block.block_meta.block_id.hash,
            time: block.block.header.time,
            txs
        }
    }
}

#[derive(Serialize, Clone)]
struct BtcBlock {
    height: u64,
    hash: String,
    num_txs: u32,
    time: u32
}

impl From<btc_json::GetBlockResult> for BtcBlock {
    fn from(block: btc_json::GetBlockResult) -> Self {
        BtcBlock {
            height: block.height as u64,
            hash: block.hash.to_string(),
            num_txs: block.n_tx as u32,
            time: block.time as u32
        }
    }
}

#[derive(Serialize)]
struct Update {
    bitcoin: Vec<BtcBlock>,
    tendermint: Vec<TmBlock>
}

struct Server {
    bitcoin_rpc: btc_rpc::Client,
    tendermint_rpc: tm_rpc::Client,
    bitcoin_blocks: Arc<Mutex<VecDeque<BtcBlock>>>,
    tendermint_blocks: Arc<Mutex<VecDeque<TmBlock>>>
}

impl Server {
    pub fn new(btc_addr: &str, tm_addr: &str) -> Result<Self> {
        let bitcoin_rpc = connect_btc_rpc(btc_addr)?;
        let tendermint_rpc = connect_tm_rpc(tm_addr)?;

        Ok(Server {
            bitcoin_rpc,
            tendermint_rpc,
            bitcoin_blocks: Default::default(),
            tendermint_blocks: Default::default()
        })
    }

    pub fn start(mut self, broadcaster: ws::Sender) {
        std::thread::spawn(move || {
            eprint!("fetching initial blocks...");
            std::io::stderr().flush().unwrap();
            self.get_new_tm_blocks(500).unwrap();
            eprintln!("done");

            loop {
                let btc_blocks = self.get_new_btc_blocks().unwrap();
                let tm_blocks = self.get_new_tm_blocks(5).unwrap();

                if !btc_blocks.is_empty() || !tm_blocks.is_empty() {
                    let update = Update {
                        bitcoin: btc_blocks,
                        tendermint: tm_blocks
                    };
                    let message = serde_json::to_string(&update).unwrap();
                    broadcaster.send(format!("{}\n", message)).unwrap();
                }

                std::thread::sleep_ms(1000);
            }
        });
    }

    fn btc_height(&self) -> u64 {
        let bitcoin_blocks = self.bitcoin_blocks.lock().unwrap();
        match bitcoin_blocks.back() {
            None => 0,
            Some(block) => block.height
        }
    }

    fn get_new_btc_blocks(
        &mut self
    ) -> Result<Vec<BtcBlock>> {
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
            blocks.push(block.clone().into());

            bitcoin_blocks.push_back(block.into());
            while bitcoin_blocks.len() > 4 {
                bitcoin_blocks.pop_front();
            }
        }

        Ok(blocks)
    }

    fn tm_height(&self) -> u64 {
        let tendermint_blocks = self.tendermint_blocks.lock().unwrap();
        match tendermint_blocks.back() {
            None => 0,
            Some(block) => block.height
        }
    }

    fn get_new_tm_blocks(&mut self, max_scan: u64) -> Result<Vec<TmBlock>> {
        let height = self.tendermint_rpc.status()?
            .sync_info.latest_block_height.value();
        
        let mut blocks = vec![];
        let start_height = max(
            self.tm_height(),
            max(height, max_scan) - max_scan
        ) + 1;

        let mut tendermint_blocks = self.tendermint_blocks.lock().unwrap();

        for h in start_height..=height {
            let res = self.tendermint_rpc.block(h)?;
            let prev = tendermint_blocks.back();

            if let Some(prev) = prev {
                if !has_header_tx(prev) && !has_header_tx(&res.clone().into()) {
                    if !blocks.is_empty() {
                        blocks.pop();
                    }
                    tendermint_blocks.pop_back();
                }
            }
            blocks.push(res.clone().into());
            tendermint_blocks.push_back(res.clone().into());

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

fn has_header_tx(block: &TmBlock) -> bool {
    for tx in block.txs.iter() {
        if let TmTx::HeaderRelay(_) = tx {
            return true;
        }
    }

    false
}
