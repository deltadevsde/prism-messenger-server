mod account;
mod database;
mod keys;
mod messages;
mod registration;
mod state;
mod webserver;

use anyhow::{Result, anyhow};
use keystore_rs::{KeyChain, KeyStore};
use log::debug;
use prism_client::{PendingTransaction as _, PrismApi as _, SigningKey};
use prism_da::{DataAvailabilityLayer, memory::InMemoryDataAvailabilityLayer};
use prism_prover::{Config, Prover, webserver::WebServerConfig as PrismWebServerConfig};
use prism_storage::inmemory::InMemoryDatabase;
use state::AppState;
use std::sync::Arc;
use tokio::spawn;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use webserver::WebServerConfig;

pub static PRISM_MESSENGER_SERVICE_ID: &str = "prism_messenger";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_env("RUST_LOG"))
        .init();

    let db = InMemoryDatabase::new();
    let (da_layer, _, _) = InMemoryDataAvailabilityLayer::new(5);

    let keystore_sk = KeyChain
        .get_or_create_signing_key(PRISM_MESSENGER_SERVICE_ID)
        .map_err(|e| anyhow!("Error getting key from store: {}", e))?;

    let sk = SigningKey::Ed25519(Box::new(keystore_sk.clone()));

    let prism_cfg = Config {
        prover: true,
        batcher: true,
        webserver: PrismWebServerConfig {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 0,
        },
        signing_key: sk.clone(),
        verifying_key: sk.verifying_key(),
        start_height: 1,
    };

    let prover = Arc::new(
        Prover::new(
            Arc::new(Box::new(db)),
            Arc::new(da_layer) as Arc<dyn DataAvailabilityLayer>,
            &prism_cfg,
        )
        .unwrap(),
    );

    let webserver_cfg = WebServerConfig {
        host: "127.0.0.1".to_string(),
        port: 48080,
    };

    let app_state = AppState::new(prover.clone(), sk.clone());

    let webserver_task_handle = spawn(async move {
        debug!("starting webserver");
        if let Err(e) = webserver::start(&webserver_cfg, app_state).await {
            log::error!("Error occurred while running prover: {:?}", e);
        }
    });

    let prover_arc = prover.clone();
    let prover_task_handle = spawn(async move {
        debug!("starting prover");
        if let Err(e) = prover_arc.run().await {
            log::error!("Error occurred while running prover: {:?}", e);
        }
    });

    register_messenger_service(prover, &sk).await?;

    tokio::select! {
        _ = prover_task_handle => {
            println!("Prover runner task completed")
        }
        _ = webserver_task_handle => {
            println!("Webserver task completed")
        }
    }

    Ok(())
}

async fn register_messenger_service(prover: Arc<Prover>, signing_key: &SigningKey) -> Result<()> {
    prover
        .register_service(
            PRISM_MESSENGER_SERVICE_ID.to_string(),
            signing_key.verifying_key(),
            signing_key,
        )
        .await
        .map_err(anyhow::Error::from)?
        .wait()
        .await
        .map_err(anyhow::Error::from)?;
    Ok(())
}
