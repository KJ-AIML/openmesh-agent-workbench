/// Best-effort redaction of common secret patterns from a session preview.
///
/// Replaces the secret value with `[REDACTED]` for a small set of common
/// token shapes. This is NOT a security boundary — the underlying files are
/// local and user-owned — but it prevents accidental exposure of credentials
/// in the UI's preview pane.
pub fn redact_secrets(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let n = chars.len();
    let mut out: Vec<char> = Vec::with_capacity(n);
    let mut i = 0;

    while i < n {
        let consumed = try_redact_at(&chars, i, &mut out);
        if consumed == 0 {
            out.push(chars[i]);
            i += 1;
        } else {
            i += consumed;
        }
    }

    out.into_iter().collect()
}

fn try_redact_at(chars: &[char], start: usize, out: &mut Vec<char>) -> usize {
    let starts = |pos: usize, needle: &[u8]| -> bool {
        if pos + needle.len() > chars.len() {
            return false;
        }
        for (offset, byte) in needle.iter().enumerate() {
            if chars[pos + offset].to_ascii_lowercase() as u8 != *byte {
                return false;
            }
        }
        true
    };

    let is_token_char = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '+' | '=' | ':');

    // sk-... / sk-proj-... / gsk_... / xai-... / gh*_... style tokens
    let prefixes: &[&[u8]] = &[
        b"sk-proj-",
        b"sk-ant-",
        b"sk-",
        b"gsk_",
        b"xai-",
        b"ghp_",
        b"gho_",
        b"github_pat_",
        b"AIza",
    ];

    for prefix in prefixes {
        if starts(start, prefix) {
            let mut end = start + prefix.len();
            while end < chars.len() && is_token_char(chars[end]) {
                end += 1;
            }
            if end - start >= prefix.len() + 8 {
                out.extend("[REDACTED]".chars());
                return end - start;
            }
        }
    }

    // Bearer <token>
    if starts(start, b"bearer ") {
        let mut end = start + "bearer ".len();
        while end < chars.len() && is_token_char(chars[end]) {
            end += 1;
        }
        if end - start > "bearer ".len() + 8 {
            out.extend("Bearer [REDACTED]".chars());
            return end - start;
        }
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_openai_style_key() {
        let out = redact_secrets("key=sk-abcdefghijklmnopqrstuvwxyz123456");
        assert!(out.contains("[REDACTED]"));
        assert!(!out.contains("sk-abcd"));
    }
}
