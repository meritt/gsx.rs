pub fn normalize_identifier(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut in_whitespace = true;
    let mut index = 0;
    let mut start = 0;

    while index < bytes.len() {
        if matches!(bytes[index], b'\t' | b'\n' | b'\r' | b' ') {
            if !in_whitespace {
                result.push_str(&value[start..index]);
                in_whitespace = true;
            }
        } else if in_whitespace {
            if !result.is_empty() {
                result.push(' ');
            }

            start = index;
            in_whitespace = false;
        }

        index += 1;
    }

    if !in_whitespace {
        result.push_str(&value[start..]);
    }

    result.to_lowercase().to_uppercase()
}
