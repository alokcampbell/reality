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
    Self { doc: Arc::new(Mutex::new(doc)), tx }
    }
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let (tx, _) = broadcast::channel(64);
        let doc = Doc::load_from_bytes(bytes)?;
        Some(Self { doc: Arc::new(Mutex::new(doc)), tx })
    }
}

#[derive(Clone)]
pub struct AppState {
    pub rooms: Arc<DashMap<String, Room>>,
}

impl AppState {
    pub fn new() -> Self {
        let rooms: Arc<DashMap<String, Room>> = Arc::new(DashMap::new());
        match std::fs::read_dir("docs") {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    match path.extension().and_then(|e| e.to_str()) {
                        Some("am") => {
                            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                match std::fs::read(&path) {
                                    // finding the docs and loading them in, maybe improve load time? make files more compact?
                                    Ok(bytes) => {
                                        match Room::from_bytes(&bytes) {
                                            Some(room) => {
                                                println!("Loaded doc (binary): {stem}");
                                                rooms.insert(stem.to_string(), room);
                                            }
                                            None => eprintln!("Failed to parse AM file: {:?}", path),
                                        }
                                    }
                                    Err(e) => eprintln!("Failed to read {:?}: {e}", path),
                                }
                            }
                        }
                        Some("md") => {
                            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                if !rooms.contains_key(stem) {
                                    match std::fs::read_to_string(&path) {
                                        Ok(content) => {
                                            println!("Loaded doc (text legacy): {stem}");
                                            let mut doc = Doc::new();
                                            if !content.is_empty() {
                                                doc.splice_text(0, 0, &content);
                                            }
                                            let (tx, _) = broadcast::channel(64);
                                            rooms.insert(stem.to_string(), Room {
                                                doc: Arc::new(Mutex::new(doc)),
                                                tx,
                                            });
                                        }
                                        Err(e) => eprintln!("Failed to read {:?}: {e}", path),
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err(_) => {
                // no doc folder found so make a new one
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
