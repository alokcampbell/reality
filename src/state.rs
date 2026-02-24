use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::crdt::Doc;

#[derive(Clone)]
pub struct Room {
    pub doc: Arc<Mutex<Doc>>,
    pub tx:  broadcast::Sender<String>,
}

impl Room {
    pub fn new(initial_text: &str) -> Self {
        let (tx, _) = broadcast::channel(64);
        let mut doc = Doc::new();
        if !initial_text.is_empty() {
            doc.splice_text(0, 0, initial_text);
        }
        Self {
            doc: Arc::new(Mutex::new(doc)),
            tx,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub rooms: Arc<DashMap<String, Room>>,
}

impl AppState {
    pub fn new() -> Self {
        let rooms: Arc<DashMap<String, Room>> = Arc::new(DashMap::new());

        // load if not in memory
        match std::fs::read_dir("docs") {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("md") {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            match std::fs::read_to_string(&path) {
                                Ok(content) => {
                                    println!("Loaded doc: {stem} ({} bytes)", content.len());
                                    rooms.insert(stem.to_string(), Room::new(&content));
                                }
                                Err(e) => eprintln!("Failed to read {:?}: {e}", path),
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // docs/ doesn't exist yet, will be created on first save
                println!("No docs/ directory found, starting fresh.");
            }
        }

        Self { rooms }
    }

    pub fn get_or_create_room(&self, doc_id: &str) -> Room {
        self.rooms
            .entry(doc_id.to_string())
            .or_insert_with(|| Room::new(""))
            .clone()
    }
}