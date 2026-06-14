use axum::{routing::get, Json, Router};
use tower_http::cors::{Any, CorsLayer};

use weft_code_runtime::{routes, service::WeftCodeService, state};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("weft_code_runtime=info".parse()?),
        )
        .init();

    let service = WeftCodeService::new(state::shared_state_with_defaults());
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .merge(routes::router())
        .layer(cors)
        .with_state(service);

    let addr = "127.0.0.1:3005";
    tracing::info!("weft-code-runtime listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
    }))
}
