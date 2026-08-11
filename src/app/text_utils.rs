//! Safe UTF-8 string manipulation utilities for text editing in TUI inputs.

/// Insert a character into `s` at the valid UTF-8 boundary at or near `byte_pos`.
/// Returns the new byte position right after the inserted character.
pub fn insert_char(s: &mut String, byte_pos: usize, c: char) -> usize {
    let target_pos = clamp_to_char_boundary(s, byte_pos);
    s.insert(target_pos, c);
    target_pos + c.len_utf8()
}

/// Delete the character preceding `byte_pos` in `s`.
/// Returns the new byte position after deletion.
pub fn delete_char_before(s: &mut String, byte_pos: usize) -> usize {
    let current_pos = clamp_to_char_boundary(s, byte_pos);
    if current_pos == 0 {
        return 0;
    }

    let prev_pos = prev_char_boundary(s, current_pos);
    s.remove(prev_pos);
    prev_pos
}

/// Get the previous valid UTF-8 character boundary before `byte_pos`.
pub fn prev_char_boundary(s: &str, byte_pos: usize) -> usize {
    let pos = clamp_to_char_boundary(s, byte_pos);
    if pos == 0 {
        return 0;
    }

    s[..pos]
        .char_indices()
        .last()
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

/// Get the next valid UTF-8 character boundary after `byte_pos`.
pub fn next_char_boundary(s: &str, byte_pos: usize) -> usize {
    let pos = clamp_to_char_boundary(s, byte_pos);
    if pos >= s.len() {
        return s.len();
    }

    s[pos..]
        .char_indices()
        .nth(1)
        .map(|(idx, _)| pos + idx)
        .unwrap_or_else(|| s.len())
}

/// Clamp a byte position so that it is guaranteed to land on a valid UTF-8 character boundary.
pub fn clamp_to_char_boundary(s: &str, byte_pos: usize) -> usize {
    if byte_pos >= s.len() {
        return s.len();
    }
    let mut pos = byte_pos;
    while pos > 0 && !s.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

/// Safely find the common prefix among strings based on valid UTF-8 byte boundaries.
pub fn safe_common_prefix(strings: &[String]) -> Option<String> {
    if strings.is_empty() {
        return None;
    }
    let first = &strings[0];
    let mut prefix_bytes = 0usize;

    'outer: for (i, c) in first.char_indices() {
        for s in &strings[1..] {
            if !s[i..].starts_with(c) {
                break 'outer;
            }
        }
        prefix_bytes = i + c.len_utf8();
    }

    if prefix_bytes > 0 {
        Some(first[..prefix_bytes].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_char_ascii() {
        let mut s = String::from("hello");
        let pos = insert_char(&mut s, 5, '!');
        assert_eq!(s, "hello!");
        assert_eq!(pos, 6);
    }

    #[test]
    fn test_insert_char_multibyte() {
        let mut s = String::from("caf");
        let pos = insert_char(&mut s, 3, 'é');
        assert_eq!(s, "café");
        assert_eq!(pos, 5); // 'é' is 2 bytes in UTF-8
    }

    #[test]
    fn test_delete_char_before_multibyte() {
        let mut s = String::from("café");
        let pos = delete_char_before(&mut s, 5);
        assert_eq!(s, "caf");
        assert_eq!(pos, 3);
    }

    #[test]
    fn test_prev_next_boundary() {
        let s = "aéñ🚀";
        // 'a' = 1b, 'é' = 2b, 'ñ' = 2b, '🚀' = 4b
        // Offsets: a(0..1), é(1..3), ñ(3..5), 🚀(5..9)
        assert_eq!(prev_char_boundary(s, 9), 5);
        assert_eq!(prev_char_boundary(s, 5), 3);
        assert_eq!(next_char_boundary(s, 1), 3);
        assert_eq!(next_char_boundary(s, 3), 5);
    }

    #[test]
    fn test_safe_common_prefix() {
        let list = vec![
            String::from("/home/user/café_repo"),
            String::from("/home/user/café_data"),
        ];
        let prefix = safe_common_prefix(&list);
        assert_eq!(prefix, Some(String::from("/home/user/café_")));
    }
}
