#![allow(dead_code)]
#![allow(unused_variables)]

use alloy::{
    eips::BlockId,
    providers::{Provider, ProviderBuilder },
};
use dotenv::dotenv;
use std::path::Path;
use anyhow::{anyhow, Result};

use block_extractor_rs::{
    tokens::*,
    pools::*,
};

#[tokio::main]
async fn main() -> Result<()> {

    dotenv().ok();
    env_logger::init();

//    let rpc_url = std::env::var("WSS_URL")?;
//    let ws = WsConnect::new(rpc_url);
//    let provider = ProviderBuilder::new().on_ws(ws).await?;

    let https_url = match std::env::var("HTTPS_URL") {
        Ok(url) => url.parse()?,
        Err(_) => return Err(anyhow!("HTTPS_URL environment variable not set")),
    };

    let provider = ProviderBuilder::new().connect_http(
        https_url
    );

    let data_dir = Path::new("./data");
    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir).expect("Failed to create data directory");
    }

    let block_number = BlockId::from(provider.get_block_number().await.unwrap());
    let from_block_number = 10000835;
    let chunks = 50000;
    let (pools, pool_id) = load_pools(
        provider.clone(),
        Path::new("./data/pools.toml"),
        from_block_number,
        chunks,
    ).await.unwrap();

    let parallel_tokens = 1;
    let tokens = load_tokens(
        provider.clone(),
        Path::new("./data/tokens.toml"),
        &pools,
        parallel_tokens,
        pool_id,
    ).await.unwrap();

    Ok(())
}
