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
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use crate::state::AppState;

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

// server and client side handling of the text changes
#[derive(Deserialize)]
struct ClientMsg {
    client_id: String,
    changes:   Vec<u8>,
}

#[derive(Serialize)]
struct ServerMsg {
    sender_id: String,
    text:      String,
    full_doc:  Vec<u8>,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let id = id.trim_start_matches('/').to_string();
    ws.on_upgrade(move |socket| handle_socket(socket, id, state))
}

// this is all the server shit when it comes to communicating the text payload, and saving the .md so it doesn't get erased in memory if the server needs a restart
async fn handle_socket(socket: WebSocket, id: String, state: AppState) {
    let room = state.get_or_create_room(&id);
    let mut rx = room.tx.subscribe();
    let (mut sink, mut stream) = socket.split();

    {
        let mut doc = room.doc.lock().await;
        let payload = serde_json::to_string(&ServerMsg {
            sender_id: "server".to_string(),
            text:      doc.get_text(),
            full_doc:  (*doc).save(),
        }).unwrap();
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

            let client_msg: ClientMsg = match serde_json::from_slice(&bytes) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[ws/{id_clone}] bad msg: {e}");
                    continue;
                }
            };

            let (new_text, full_doc) = {
                let mut doc = room_clone.doc.lock().await;
                let text = doc.merge_changes(&client_msg.changes);
                eprintln!("[ws/{id_clone}] merged, text len={}", text.len());
                let full = (*doc).save();
                (text, full)
            };

            let text_for_save    = new_text.clone();
            let full_doc_for_save = full_doc.clone();
            let id_for_save      = id_clone.clone();
            tokio::spawn(async move {
                if let Err(e) = std::fs::create_dir_all("docs") {
                    eprintln!("create_dir_all failed: {e}");
                    return;
                }
                let path = format!("docs/{id_for_save}.md");
                if let Err(e) = std::fs::write(&path, &text_for_save) {
                    eprintln!("write {path} failed: {e}");
                }
                let am_path = format!("docs/{id_for_save}.am");
                if let Err(e) = std::fs::write(&am_path, &full_doc_for_save) {
                    eprintln!("write {am_path} failed: {e}");
                }
            });

            let payload = serde_json::to_string(&ServerMsg {
                sender_id: client_msg.client_id,
                text:      new_text,
                full_doc,
            }).unwrap();
            let _ = room_clone.tx.send(payload);
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}