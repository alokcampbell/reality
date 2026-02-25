use automerge::{AutoCommit, ObjType, ReadDoc};
use automerge::transaction::Transactable;

pub struct Doc {
    am: AutoCommit,
    text_obj: automerge::ObjId,
}

// serverside of the crdt, automerges
impl Doc {
    pub fn new() -> Self {
        let mut am = AutoCommit::new();
        let text_obj = am
            .put_object(automerge::ROOT, "text", ObjType::Text)
            .unwrap();
        Self { am, text_obj }
    }

    pub fn get_text(&self) -> String {
        self.am.text(&self.text_obj).unwrap_or_default()
    }
    // reads and updates the text
    pub fn splice_text(&mut self, insert_at: usize, delete_count: usize, insert: &str) -> String {
        self.am
            .splice_text(&self.text_obj, insert_at, delete_count as isize, insert)
            .unwrap();
        self.get_text()
    }

    pub fn save_changes(&mut self) -> Vec<u8> {
        self.am.save_incremental()
    }

    pub fn save(&mut self) -> Vec<u8> {
        self.am.save()
    }

    pub fn load_from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut am = AutoCommit::load(bytes).ok()?;
        let text_obj = am.get(automerge::ROOT, "text").ok()??.1;
        Some(Self { am, text_obj })
    }

    // new merger, let's see if its better then my shitty one from before

    pub fn merge_changes(&mut self, bytes: &[u8]) -> String {
        if let Ok(mut other) = AutoCommit::load(bytes) {
            let _ = self.am.merge(&mut other);
        }
        self.get_text()
    }
}