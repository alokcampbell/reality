// clientside of the crdt

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
