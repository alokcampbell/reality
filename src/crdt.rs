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

    pub fn splice_text(&mut self, index: usize, delete: usize, insert: &str) -> String {
        self.am
            .splice_text(&self.text_obj, index, delete as isize, insert)
            .unwrap();
        self.get_text()
    }
}