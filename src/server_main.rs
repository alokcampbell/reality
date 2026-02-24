mod crdt;
mod state;
mod ws;

use axum::Router;
use tower_http::{cors::CorsLayer, services::ServeDir};
// server backend
#[tokio::main]
async fn main() {
    let state = state::AppState::new();
    let ws_routes = ws::ws_router(state);

    let app = Router::new()
        .merge(ws_routes)
        .fallback_service(ServeDir::new("target/dx/reality/release/web/public"))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    println!("Reality running on http://0.0.0.0:3001");
    axum::serve(listener, app).await.unwrap();
}