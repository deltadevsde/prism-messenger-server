use anyhow::{Context, Result};
use log::info;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa::{
    OpenApi,
    openapi::{Info, OpenApiBuilder},
};
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

use crate::{account, keys, messages, registration, settings::WebserverSettings, state::AppState};

#[derive(OpenApi)]
struct ApiDoc;

pub async fn start(settings: &WebserverSettings, state: AppState) -> Result<()> {
    let state_arc = Arc::new(state);
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/accounts", account::router())
        .nest("/keys", keys::router(state_arc.clone()))
        .nest("/messages", messages::router(state_arc.clone()))
        .nest("/registration", registration::router())
        .with_state(state_arc)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .split_for_parts();

    let api = OpenApiBuilder::from(api)
        .info(Info::new("Prism Messenger Server API", "0.1.0"))
        .build();

    let router = router.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api));

    let addr = SocketAddr::new(
        settings.host.parse().expect("IP address can be parsed"),
        settings.port,
    );
    let listener = TcpListener::bind(addr)
        .await
        .expect("Binding to address works");
    let server = axum::serve(listener, router.into_make_service());

    let socket_addr = server.local_addr()?;
    info!(
        "Starting webserver on {}:{}",
        settings.host,
        socket_addr.port()
    );

    server.await.context("Server error")?;

    Ok(())
}
