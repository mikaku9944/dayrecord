//! Redact sensitive patterns before data leaves the machine (API prompts / exports).
//! Manual scanners instead of the regex crate to keep core dependency-free.

pub fn sanitize(text: &str) -> String {
    let mut out = text.to_string();
    out = replace_phone(&out, "[PHONE]");
    out = replace_id(&out, "[ID]");
    out = replace_email(&out, "[EMAIL]");
    out = replace_long_digits(&out, "[REDACTED]");
    out = replace_sk_key(&out, "[API_KEY]");
    out = replace_bearer(&out, "Bearer [TOKEN]");
    out
}

/// Chinese mobile: 11 digits starting with 1[3-9].
fn replace_phone(input: &str, rep: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '1' && i + 10 < chars.len() {
            let slice: String = chars[i..i + 11].iter().collect();
            if slice.chars().all(|c| c.is_ascii_digit()) {
                let second = slice.chars().nth(1).unwrap_or('0');
                if ('3'..='9').contains(&second) {
                    out.push_str(rep);
                    i += 11;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Chinese ID card: 17 digits + digit/X.
fn replace_id(input: &str, rep: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if i + 17 < chars.len() {
            let slice = &chars[i..i + 18];
            if slice[..17].iter().all(|c| c.is_ascii_digit()) {
                let last = slice[17];
                if last.is_ascii_digit() || last == 'X' || last == 'x' {
                    out.push_str(rep);
                    i += 18;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn replace_email(input: &str, rep: &str) -> String {
    let mut out = input.to_string();
    for word in input.split_whitespace() {
        if let Some(at) = word.find('@') {
            let local = &word[..at];
            let domain = &word[at + 1..];
            if !local.is_empty()
                && domain.contains('.')
                && domain.chars().all(|c| c.is_ascii_alphanumeric() || ".-_".contains(c))
            {
                out = out.replace(word, rep);
            }
        }
    }
    out
}

/// Card numbers, tokens: any run of 12+ digits.
fn replace_long_digits(input: &str, rep: &str) -> String {
    let mut out = String::new();
    let mut digit_run = String::new();
    for ch in input.chars() {
        if ch.is_ascii_digit() {
            digit_run.push(ch);
        } else {
            if digit_run.len() >= 12 {
                out.push_str(rep);
            } else {
                out.push_str(&digit_run);
            }
            digit_run.clear();
            out.push(ch);
        }
    }
    if digit_run.len() >= 12 {
        out.push_str(rep);
    } else {
        out.push_str(&digit_run);
    }
    out
}

fn replace_sk_key(input: &str, rep: &str) -> String {
    let mut out = input.to_string();
    for word in input.split_whitespace() {
        if word.starts_with("sk-") && word.len() > 23 {
            out = out.replace(word, rep);
        }
    }
    out
}

fn replace_bearer(input: &str, rep: &str) -> String {
    if let Some(idx) = input.find("Bearer ") {
        let after = idx + "Bearer ".len();
        let tail = &input[after..];
        let token_len = tail.find(char::is_whitespace).unwrap_or(tail.len());
        let full = &input[idx..after + token_len];
        return input.replace(full, rep);
    }
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_phone() {
        assert!(sanitize("call 13812345678").contains("[PHONE]"));
    }

    #[test]
    fn redacts_email() {
        assert!(sanitize("mail me at someone@example.com please").contains("[EMAIL]"));
    }

    #[test]
    fn redacts_sk_key() {
        assert!(sanitize("key sk-abcdefghijklmnopqrstuvwxyz123").contains("[API_KEY]"));
    }

    #[test]
    fn redacts_bearer_token() {
        assert!(sanitize("Authorization: Bearer abc.def-123").contains("Bearer [TOKEN]"));
    }

    #[test]
    fn sanitize_chinese_text_no_panic() {
        let text = "界面文本快照：密码框输入测试内容，不会在中文字符处崩溃";
        let _ = sanitize(text);
    }
}
