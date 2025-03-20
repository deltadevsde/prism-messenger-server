use anyhow::{Context, Result};
use log::info;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa::{
    OpenApi,
    openapi::{Info, OpenApiBuilder},
};
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

use crate::{account, keys, messages, registration, state::AppState};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for WebServerConfig {
    fn default() -> Self {
        WebServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
        }
    }
}

#[derive(OpenApi)]
struct ApiDoc;

pub async fn start(config: &WebServerConfig, state: AppState) -> Result<()> {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/accounts", account::router())
        .nest("/keys", keys::router())
        .nest("/messages", messages::router())
        .nest("/registration", registration::router())
        .with_state(Arc::new(state))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .split_for_parts();

    let api = OpenApiBuilder::from(api)
        .info(Info::new("Prism Messenger Server API", "0.1.0"))
        .build();

    let router = router.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api));

    let addr = SocketAddr::new(
        config.host.parse().expect("IP address can be parsed"),
        config.port,
    );
    let listener = TcpListener::bind(addr)
        .await
        .expect("Binding to address works");
    let server = axum::serve(listener, router.into_make_service());

    let socket_addr = server.local_addr()?;
    info!(
        "Starting webserver on {}:{}",
        config.host,
        socket_addr.port()
    );

    server.await.context("Server error")?;

    Ok(())
}
