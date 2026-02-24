mod crdt;
mod state;
mod ws;
// this hosts the database
#[tokio::main]
async fn main() {
    let state = state::AppState::new();
    let router = ws::ws_router(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    println!("WS server listening on ws://0.0.0.0:3001");
    axum::serve(listener, router).await.unwrap();
}