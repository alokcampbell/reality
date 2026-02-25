use automerge::{AutoCommit, ObjType, ReadDoc};
use automerge::transaction::Transactable;

pub struct Doc {
    am: AutoCommit,
    text_obj: automerge::ObjId,
}

// clientside of the crdt
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

    pub fn splice_text(&mut self, insert_at: usize, delete_count: usize, insert: &str) {
        self.am
            .splice_text(&self.text_obj, insert_at, delete_count as isize, insert)
            .unwrap();
    }

    pub fn save_changes(&mut self) -> Vec<u8> {
        self.am.save()
    }

    pub fn merge_from_bytes(&mut self, bytes: &[u8]) -> Option<String> {
        let mut other = AutoCommit::load(bytes).ok()?;
        self.am.merge(&mut other).ok()?;
        Some(self.get_text())
    }

    pub fn load_from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut am = AutoCommit::load(bytes).ok()?;
        let text_obj = am.get(automerge::ROOT, "text").ok()??.1;
        Some(Self { am, text_obj })
    }
}

pub fn diff(old: &str, new: &str) -> (usize, usize, String) {
    let old_chars: Vec<char> = old.chars().collect();
    let new_chars: Vec<char> = new.chars().collect();
    // changing to vectors
    let prefix = old_chars
        .iter()
        .zip(new_chars.iter())
        .take_while(|(a, b)| a == b)
        .count();
    // next is a method to try to speed up realization time by not checking the whole .md file
    let old_suffix = &old_chars[prefix..];
    let new_suffix = &new_chars[prefix..];
    let suffix = old_suffix
        .iter()
        .rev()
        .zip(new_suffix.iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let delete = old_suffix.len() - suffix;
    let insert: String = new_suffix[..new_suffix.len() - suffix].iter().collect();
    (prefix, delete, insert)
}