use crate::document::Code;

const SAFE: i128 = 9_007_199_254_740_991;

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn digits(value: &str) -> Option<i64> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn ends_with_zone(value: &str) -> bool {
    let bytes = value.as_bytes();
    value.ends_with(['Z', 'z'])
        || (bytes.len() >= 6 && {
            let zone = &bytes[bytes.len() - 6..];
            matches!(zone[0], b'+' | b'-')
                && zone[1..3].iter().all(u8::is_ascii_digit)
                && zone[3] == b':'
                && zone[4..].iter().all(u8::is_ascii_digit)
        })
}

pub(super) fn absolute(value: &str) -> Result<i64, Code> {
    let fail = || {
        let bytes = value.as_bytes();
        let date_t = bytes.len() >= 11
            && value.is_ascii()
            && digits(&value[0..4]).is_some()
            && bytes[4] == b'-'
            && digits(&value[5..7]).is_some()
            && bytes[7] == b'-'
            && digits(&value[8..10]).is_some()
            && matches!(bytes[10], b'T' | b't');
        if date_t && !ends_with_zone(value) { Code::DatetimeTimezoneRequired } else { Code::DatetimeInvalid }
    };
    if !value.is_ascii() || value.len() < 20 {
        return Err(fail());
    }
    let bytes = value.as_bytes();
    let field = |range: std::ops::Range<usize>| digits(&value[range]);
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) =
        (field(0..4), field(5..7), field(8..10), field(11..13), field(14..16), field(17..19))
    else {
        return Err(fail());
    };
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || !matches!(bytes[10], b'T' | b't')
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return Err(fail());
    }
    let mut rest = &value[19..];
    let mut millis = 0;
    if let Some(fraction) = rest.strip_prefix('.') {
        let length = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if length == 0 {
            return Err(fail());
        }
        let head = &fraction[..length.min(3)];
        millis = digits(head).unwrap_or(0) * 10_i64.pow(3 - head.len() as u32);
        rest = &fraction[length..];
    }
    let offset_minutes = match rest {
        "Z" | "z" => 0,
        zone if zone.len() == 6 && matches!(zone.as_bytes()[0], b'+' | b'-') && zone.as_bytes()[3] == b':' => {
            let (Some(hours), Some(minutes)) = (digits(&zone[1..3]), digits(&zone[4..6])) else {
                return Err(fail());
            };
            if zone == "-00:00" {
                return Err(Code::DatetimeTimezoneRequired);
            }
            if hours > 23 || minutes > 59 {
                return Err(Code::DatetimeInvalid);
            }
            (hours * 60 + minutes) * if zone.starts_with('+') { 1 } else { -1 }
        }
        _ => return Err(fail()),
    };
    if !(1..=12).contains(&month)
        || day < 1
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return Err(Code::DatetimeInvalid);
    }
    let ms =
        days_from_civil(year, month, day) * 86_400_000 + hour * 3_600_000 + minute * 60_000 + second * 1000 + millis
            - offset_minutes * 60_000;
    Ok(ms.div_euclid(1000))
}

pub(super) fn unix(value: &str) -> Result<i64, Code> {
    let digits = value.strip_prefix('-').unwrap_or(value);
    let canonical = digits == "0"
        || (!digits.is_empty() && !digits.starts_with('0') && digits.bytes().all(|byte| byte.is_ascii_digit()));
    if !canonical || digits.len() > 20 {
        return Err(Code::DatetimeInvalid);
    }
    let number: i128 = value.parse().map_err(|_| Code::DatetimeInvalid)?;
    if number.abs() > SAFE {
        return Err(Code::DatetimeInvalid);
    }
    i64::try_from(number).map_err(|_| Code::DatetimeInvalid)
}

pub(super) fn reserved_offset(raw: &str) -> Option<usize> {
    let bytes = raw.as_bytes();
    let mut slashes = 0;
    for (index, &byte) in bytes.iter().enumerate() {
        if byte == b'\\' {
            slashes += 1;
            continue;
        }
        if byte == b':'
            && slashes % 2 == 0
            && raw[index..].starts_with("::time")
            && !bytes.get(index + 6).is_some_and(|next| next.is_ascii_alphanumeric() || b"_.-".contains(next))
        {
            return Some(index);
        }
        slashes = 0;
    }
    None
}

fn closing_end(bytes: &[u8], start: usize, close: u8) -> usize {
    let mut quote = 0;
    let mut index = start;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'\n' {
            return index;
        }
        if byte == b'\\' {
            index += 1;
        } else if quote != 0 {
            if byte == quote {
                quote = 0;
            }
        } else if byte == b'"' || byte == b'\'' {
            quote = byte;
        } else if byte == close {
            return index + 1;
        }
        index += 1;
    }
    bytes.len()
}

pub(super) fn reserved_end(bytes: &[u8], start: usize) -> usize {
    let mut end = start + 6;
    if bytes.get(end) == Some(&b'[') {
        end = closing_end(bytes, end + 1, b']');
        if bytes.get(end.wrapping_sub(1)) != Some(&b']') {
            return end;
        }
    }
    if bytes.get(end) == Some(&b'{') {
        return closing_end(bytes, end + 1, b'}');
    }
    end
}
