use std::{
    collections::BTreeMap,
    fs::OpenOptions,
    io::Write,
    path::Path,
};
use alloy::{
    primitives::Address,
    providers::Provider,
};
use indicatif::{ProgressBar, ProgressStyle};
use anyhow::{anyhow, Result};
use log::info;
use serde::{Serialize, Deserialize};

use crate::{interfaces::IERC20, pools::Pool};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub address: Address,
    pub name: String,
    pub symbol: String,
    pub decimals: u8
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokensToml {
    token: Vec<Token>,
}

pub async fn load_tokens(
    provider: impl Provider + 'static + Clone,
    path: &Path,
    pools: &BTreeMap<Address, Pool>,
    parallel: u64,
    last_pool_id: i64,
) -> Result<BTreeMap<Address, Token>> {

    info!("Loading tokens...");

    let mut tokens = load_tokens_from_file(path)?;


    let pb = ProgressBar::new(pools.len() as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )
        .unwrap()
        .progress_chars("##-"),
    );


    let mut count = 0;
    let mut requests = Vec::new();

    for (_, pool) in pools.into_iter() {
        let pool_id = pool.id;
        if pool_id <= last_pool_id {
            continue;
        }
        let token0 = pool.token0;
        let token1 = pool.token1;
        for token in [token0, token1] {
            if !tokens.contains_key(&token) {
                requests.push(
                    tokio::task::spawn(
                        get_token_data(
                            provider.clone(),
                            token,
                        )
                    )
                );
                count += 1 ;
            }
            if count == parallel {
                let results = futures::future::join_all(requests).await;
                for result in results {
                    match result {
                        Ok(r) => match r {
                            Ok(t) => {
                                tokens.insert(
                                    t.address,
                                    Token {
                                        address: t.address,
                                        name: t.name,
                                        symbol: t.symbol,
                                        decimals: t.decimals
                                    }
                                );
                            }
                            Err(e) => { info!("Something wrong 0 {:?}", e) }
                        }
                        Err(e) => { info!("Something wrong 1 {:?}", e) }
                    }
                }
                requests = Vec::new();
                count = 0;
                pb.inc(parallel);
            }
        }
    }

    write_tokens_to_toml(&tokens, path)?;

    Ok(tokens)
}

async fn get_token_data(
    provider: impl Provider,
    token: Address,
) -> Result<Token> {

    let interface = IERC20::new(token, provider);

    let decimals = match interface.decimals().call().await {
        Ok(r) => r,
        Err(e) => { return Err(anyhow!("Decimals of token failed {:?}", e )) }
    };

    let name = match interface.name().call().await {
        Ok(r) => r,
        Err(e) => {
            info!("Name of token {:?} failed {:?}", token, e);
            String::from("PlaceHolderName")
        }
    };
    let symbol = match interface.symbol().call().await{
        Ok(r) => r,
        Err(e) => {
            info!("Symbol of token failed {:?}", e );
            String::from("PlaceHolderSymbol")
        }
    };

    Ok(Token {
        address: token,
        name,
        symbol,
        decimals,
    })
}

pub fn load_tokens_from_file(
    path: &Path,
) -> Result<BTreeMap<Address, Token>> {
    let mut tokens = BTreeMap::new();

    if path.exists() {
        let reader = std::fs::read_to_string(path)?;
        let tokens_toml: TokensToml = toml::from_str(&reader)?;
        for token in tokens_toml.token {
            tokens.insert(token.address, token);
        }
    }

    Ok(tokens)
}

pub fn write_tokens_to_toml(
    tokens: &BTreeMap<Address, Token>,
    path: &Path,
) -> Result<()> {
    let tokens_vec: Vec<Token> = tokens.values().cloned().collect();
    let tokens_toml = TokensToml { token: tokens_vec };

    let toml_string = toml::to_string_pretty(&tokens_toml)?;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;

    file.write_all(toml_string.as_bytes())?;

    Ok(())
}
