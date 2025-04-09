mod account;
mod crypto;
mod database;
mod initialization;
mod keys;
mod messages;
mod notifications;
mod registration;
mod settings;
mod state;
mod webserver;

use anyhow::Result;
use settings::Settings;
use state::AppState;
use std::error::Error;
use tokio::spawn;
use tracing::debug;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub static PRISM_MESSENGER_SERVICE_ID: &str = "prism_messenger";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_env("RUST_LOG"))
        .init();

    let settings = Settings::load()?;
    let app_state = AppState::from_settings(&settings)?;

    app_state
        .initialization_service
        .initialize_messenger_server()
        .await?;

    let webserver_task_handle = spawn(async move {
        debug!("starting webserver");
        if let Err(e) = webserver::start(&settings.webserver, app_state).await {
            log::error!("Error occurred while running prover: {:?}", e);
        }
    });

    tokio::select! {
        _ = webserver_task_handle => {
            println!("Webserver task completed")
        }
    }

    Ok(())
}
