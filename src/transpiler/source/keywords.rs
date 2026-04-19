pub fn rewrite_word(source: &str, from: &str, to: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if starts_with_keyword(source, index, from) {
            result.push_str(to);
            index += from.len();
            continue;
        }

        result.push(bytes[index] as char);
        index += 1;
    }

    result
}

pub(crate) fn starts_with_keyword(source: &str, index: usize, keyword: &str) -> bool {
    source[index..].starts_with(keyword)
        && source[..index]
            .chars()
            .next_back()
            .is_none_or(|ch| !is_identifier_continue(ch))
        && source[index + keyword.len()..]
            .chars()
            .next()
            .is_none_or(|ch| !is_identifier_continue(ch))
}

pub(crate) fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

pub(crate) fn is_identifier_continue(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}

pub(crate) fn scan_identifier(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    let mut index = start;
    while index < bytes.len() && is_identifier_continue(bytes[index] as char) {
        index += 1;
    }
    index
}
