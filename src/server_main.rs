mod crdt;
mod state;
mod ws;

use axum::Router;
use tower_http::{cors::CorsLayer, services::{ServeDir, ServeFile}};

#[tokio::main]
async fn main() {
    let state = state::AppState::new();
    let ws_routes = ws::ws_router(state);

    let serve_dir = ServeDir::new("target/dx/reality/release/web/public")
        .fallback(ServeFile::new("target/dx/reality/release/web/public/index.html")); //allows full code urls to work, no idea why

    let app = Router::new()
        .merge(ws_routes)
        .fallback_service(serve_dir)
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    println!("Reality running on http://0.0.0.0:3001");
    axum::serve(listener, app).await.unwrap();
}