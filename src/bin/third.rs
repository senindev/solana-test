use futures::stream::StreamExt;
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use serde_yaml;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signer::Signer, system_instruction,
    transaction::Transaction,
};
use std::fs;
use std::{error::Error, str::FromStr};
use tonic::transport::channel::ClientTlsConfig;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::prelude::{SubscribeRequest, SubscribeRequestFilterBlocksMeta};

#[derive(Deserialize, Serialize, Debug)]
struct Config {
    wallet: WalletConfig,
    geyser: GeyserConfig,
}

#[derive(Deserialize, Serialize, Debug)]
struct WalletConfig {
    rpc_url: String,
    secret_key: String,
    to: String,
    amount: u64,
}

#[derive(Deserialize, Serialize, Debug)]
struct GeyserConfig {
    url: String,
    token: String,
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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config: Config = load_config("config.yaml")?;

    // Инициализация Rpc клиента. Можно сделать через Arc как в прошлых задачах
    let client: &'static RpcClient = Box::leak(Box::new(RpcClient::new(config.wallet.rpc_url)));
    let sender_keypair: &'static Keypair = Box::leak(Box::new(Keypair::from_base58_string(
        &config.wallet.secret_key,
    )));
    let recipient_pubkey: &'static Pubkey =
        Box::leak(Box::new(Pubkey::from_str(&config.wallet.to).unwrap()));

    // Инициализация Geyser
    let mut geyser_client = GeyserGrpcClient::build_from_shared(config.geyser.url)?
        .x_token(Some(config.geyser.token))?
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .connect()
        .await?;
    let (_, mut stream) = geyser_client
        .subscribe_with_request(Some(SubscribeRequest {
            blocks_meta: hashmap! {"client".to_owned() => SubscribeRequestFilterBlocksMeta{}},
            ..Default::default()
        }))
        .await?;
    let amount_sol = config.wallet.amount as f64 / 1_000_000_000f64;
    while let Some(response) = stream.next().await {
        match response {
            Ok(event) => {
                if let Some(upd) = event.update_oneof {
                    if let UpdateOneof::BlockMeta(block) = upd {
                        println!("New block! Hash: {}, Slot: {}", block.blockhash, block.slot)
                    }
                }
                match transfer(
                    client,
                    sender_keypair,
                    recipient_pubkey,
                    config.wallet.amount,
                )
                .await
                {
                    Ok(hash) => {
                        println!(
                            "Successfully transfer {} SOL from {} to {}. TX hash: {}",
                            amount_sol,
                            sender_keypair.pubkey().to_string(),
                            recipient_pubkey.to_string(),
                            hash
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to transfer {} SOL from {} to {}",
                            amount_sol,
                            sender_keypair.pubkey().to_string(),
                            recipient_pubkey.to_string()
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("Block retrieval error: {:?}", e);
            }
        }
    }
    Ok(())
}
