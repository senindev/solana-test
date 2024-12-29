use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::{error::Error, fs, sync::Arc};
use tokio;

#[derive(Deserialize, Debug, Clone)]
struct Config {
    rpc_url: String,
    wallets: Vec<String>,
}

fn load_config(filename: &str) -> Result<Config, Box<dyn Error>> {
    let config_str = fs::read_to_string(filename)?;
    let config: Config = serde_yaml::from_str(&config_str)?;
    Ok(config)
}

async fn get_balance(client: &RpcClient, wallet: &str) -> Result<u64, Box<dyn Error>> {
    let pubkey = wallet.parse::<Pubkey>()?;
    let balance = client.get_balance(&pubkey)?;
    Ok(balance)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = load_config("config.yaml")?;

    let client = Arc::new(RpcClient::new(config.rpc_url));
    // Не оптимизировано под большое количество кошельков. Для тестового это не требуется
    let mut handles = vec![];
    for wallet in config.wallets {
        let client = Arc::clone(&client);
        handles.push(tokio::spawn(async move {
            match get_balance(&client, &wallet).await {
                Ok(balance) => {
                    let balance_sol = balance as f64 / 1_000_000_000f64;
                    println!("Wallet {}: {} SOL", wallet, balance_sol);
                }
                Err(e) => eprintln!("Failed to fetch balance for {}: {}", wallet, e),
            }
        }));
    }
    for handle in handles {
        handle.await?;
    }

    Ok(())
}
