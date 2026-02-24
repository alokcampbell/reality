use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tower_http::cors::{Any, CorsLayer};

use crate::state::AppState;

// websocket logic, router, etc


pub fn ws_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    Router::new()
        .route("/ws/*id", get(ws_handler))
        .layer(cors)
        .with_state(state)
}

#[derive(Deserialize)]
struct SpliceMsg {
    index: usize,
    delete: usize,
    insert: String,
}

#[derive(Deserialize)]
struct ClientMsg {
    splice: SpliceMsg,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let id = id.trim_start_matches('/').to_string();
    ws.on_upgrade(move |socket| handle_socket(socket, id, state))
}

// logic for message and lobby replication

async fn handle_socket(socket: WebSocket, id: String, state: AppState) {
    let room = state.get_or_create_room(&id);
    let mut rx = room.tx.subscribe();
    let (mut sink, mut stream) = socket.split();

    {
        let doc = room.doc.lock().await;
        let payload = make_text_payload(&doc.get_text());
        let _ = sink.send(Message::Text(payload.into())).await;
    }

    let mut send_task = tokio::spawn(async move {
        while let Ok(payload) = rx.recv().await {
            if sink.send(Message::Text(payload.into())).await.is_err() {
                break;
            }
        }
    });

    let room_clone = room.clone();
    let id_clone = id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            let bytes = match msg {
                Message::Text(t)   => t.into_bytes(),
                Message::Binary(b) => b.to_vec(),
                Message::Close(_)  => break,
                _                  => continue,
            };

            let cmd: ClientMsg = match serde_json::from_slice(&bytes) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[ws/{id_clone}] bad msg: {e}  raw: {}", String::from_utf8_lossy(&bytes));
                    continue;
                }
            };

            let new_text = {
                let mut doc = room_clone.doc.lock().await;
                doc.splice_text(cmd.splice.index, cmd.splice.delete, &cmd.splice.insert)
            };

            let text_for_save = new_text.clone();
            let id_for_save   = id_clone.clone();
            tokio::spawn(async move {
                if let Err(e) = std::fs::create_dir_all("docs") {
                    eprintln!("create_dir_all failed: {e}");
                    return;
                }
                let path = format!("docs/{id_for_save}.md");
                if let Err(e) = std::fs::write(&path, &text_for_save) {
                    eprintln!("write {path} failed: {e}");
                }
            });

            let _ = room_clone.tx.send(make_text_payload(&new_text));
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}

// manages new messages

fn make_text_payload(text: &str) -> String {
    format!(r#"{{"text":{}}}"#, serde_json::to_string(text).unwrap())
}