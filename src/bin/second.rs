use serde::Deserialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signer::Signer, system_instruction,
    transaction::Transaction,
};
use std::{error::Error, fs, str::FromStr, sync::Arc};
use tokio;

#[derive(Deserialize, Debug, Clone)]
struct Transfer {
    secret_key: String,
    to: String,
    amount: u64,
}

#[derive(Deserialize, Debug, Clone)]
struct Config {
    rpc_url: String,
    transfers: Vec<Transfer>,
}

fn load_config(filename: &str) -> Result<Config, Box<dyn Error>> {
    let config_str = fs::read_to_string(filename)?;
    let config: Config = serde_yaml::from_str(&config_str)?;
    Ok(config)
}

async fn transfer(
    client: &RpcClient,
    sender: &Keypair,
    to: &Pubkey,
    amount: u64,
) -> Result<String, Box<dyn Error>> {
    let transaction = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(&sender.pubkey(), to, amount)],
        Some(&sender.pubkey()),
        &[sender],
        client.get_latest_blockhash().await?,
    );
    let signature = client.send_and_confirm_transaction(&transaction).await?;
    Ok(signature.to_string())
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = load_config("config.yaml")?;
    dbg!(config.clone());

    let client = Arc::new(RpcClient::new(config.rpc_url));
    let mut handles = vec![];
    for t in config.transfers {
        let client = Arc::clone(&client);
        let sender_keypair = Keypair::from_base58_string(&t.secret_key);
        let recipient_pubkey = Pubkey::from_str(&t.to)?;
        let balance_sol = t.amount as f64 / 1_000_000_000f64;
        handles.push(tokio::spawn(async move {
            match transfer(&client, &sender_keypair, &recipient_pubkey, t.amount).await {
                Ok(hash) => {
                    println!(
                        "Successfully transfer {} SOL from {} to {}. TX hash: {}",
                        balance_sol,
                        sender_keypair.pubkey().to_string(),
                        recipient_pubkey.to_string(),
                        hash
                    );
                }
                Err(e) => {
                    eprintln!(
                        "Failed to transfer {} SOL from {} to {}",
                        balance_sol,
                        sender_keypair.pubkey().to_string(),
                        recipient_pubkey.to_string()
                    );
                }
            }
        }));
    }
    for handle in handles {
        handle.await?
    }
    Ok(())
}
