#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchRange {
    pub start: usize,
    pub end: usize,
}

pub fn find_document_match(
    text: &str,
    query: &str,
    from_offset: usize,
    forward: bool,
) -> Option<MatchRange> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return None;
    }

    let text_chars: Vec<char> = text.chars().collect();
    let needle: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();
    let mut folded = Vec::new();
    let mut original_indices = Vec::new();
    for (index, ch) in text_chars.iter().enumerate() {
        for lower in ch.to_lowercase() {
            folded.push(lower);
            original_indices.push(index);
        }
    }
    if needle.len() > folded.len() {
        return None;
    }
    let candidates = (0..=folded.len() - needle.len())
        .filter(|index| folded[*index..*index + needle.len()] == needle)
        .collect::<Vec<_>>();
    let selected = if forward {
        candidates
            .iter()
            .copied()
            .find(|index| original_indices[*index] >= from_offset)
            .or_else(|| candidates.first().copied())
    } else {
        candidates
            .iter()
            .copied()
            .rev()
            .find(|index| original_indices[*index] < from_offset)
            .or_else(|| candidates.last().copied())
    }?;
    let start = original_indices[selected];
    let end = original_indices[selected + needle.len() - 1] + 1;
    Some(MatchRange { start, end })
}
