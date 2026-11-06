fn is_ws(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r')
}

pub fn data_pipes(line: &[u8], columns: usize, exact: bool) -> Vec<usize> {
    if !line.windows(2).any(|w| w == b"||") {
        return vec![];
    }
    let escaped = escaped_pipes(line);
    let mut inactive = vec![false; line.len()];
    mark_code_spans(line, &mut inactive);
    let angles = line.contains(&b'<').then(|| angle_candidates(line));
    if let Some(angles) = &angles {
        mark_angle_contexts(line, &mut inactive, angles);
    }
    if line.windows(2).any(|w| w == b"](") {
        mark_explicit_destinations(line, &mut inactive, angles.as_ref());
    }
    mark_bare_autolinks(line, &mut inactive);
    let hidden = spoiler_pipes(line, &escaped, &inactive);
    if hidden.is_empty() || cell_count(line, &escaped, &[]) <= columns {
        return vec![];
    }
    let cells = cell_count(line, &escaped, &hidden);
    if (exact && cells == columns) || (!exact && cells <= columns) { hidden } else { vec![] }
}

pub fn delimiter_columns(line: &[u8]) -> Option<usize> {
    let escaped = escaped_pipes(line);
    let found = pipes(line, &escaped, &[]);
    if found.is_empty() {
        return None;
    }
    let (first, last) = bounds(line);
    let content_start = if found[0] as i64 == first { first + 1 } else { first };
    let content_end = if *found.last().unwrap() as i64 == last { last } else { last + 1 };
    let mut cells: Vec<&[u8]> = vec![];
    let mut start = content_start;
    for &pipe in &found {
        let pipe = pipe as i64;
        if pipe >= content_start && pipe < content_end {
            cells.push(line[start as usize..pipe as usize].trim_ascii());
            start = pipe + 1;
        }
    }
    let tail = if content_end >= start { &line[start as usize..content_end as usize] } else { &[][..] };
    cells.push(tail.trim_ascii());
    let is_delimiter = |cell: &&[u8]| {
        let inner = cell.strip_prefix(b":").unwrap_or(cell);
        let inner = inner.strip_suffix(b":").unwrap_or(inner);
        !inner.is_empty() && inner.iter().all(|b| *b == b'-')
    };
    cells.iter().all(is_delimiter).then_some(cells.len())
}

fn spoiler_pipes(line: &[u8], escaped: &[bool], inactive: &[bool]) -> Vec<usize> {
    let mut hidden = vec![];
    let mut opening: Option<usize> = None;
    let mut offset = 0;
    while offset + 1 < line.len() {
        if line[offset] != b'|'
            || line[offset + 1] != b'|'
            || escaped[offset]
            || escaped[offset + 1]
            || inactive[offset]
            || inactive[offset + 1]
        {
            offset += 1;
            continue;
        }
        let can_open = offset + 2 < line.len() && !is_ws(line[offset + 2]);
        let can_close = offset > 0 && !is_ws(line[offset - 1]);
        if let (true, Some(open)) = (can_close, opening) {
            hidden.extend([open, open + 1, offset, offset + 1]);
            opening = None;
        } else if can_open {
            opening = Some(offset);
        }
        offset += 2;
    }
    hidden
}

fn escaped_pipes(line: &[u8]) -> Vec<bool> {
    let mut result = vec![false; line.len()];
    let mut slashes = 0;
    for (offset, &byte) in line.iter().enumerate() {
        if byte == b'\\' {
            slashes += 1;
            continue;
        }
        if byte == b'|' && slashes % 2 == 1 {
            result[offset] = true;
        }
        slashes = 0;
    }
    result
}

fn mark_code_spans(line: &[u8], inactive: &mut [bool]) {
    struct Run {
        start: usize,
        end: usize,
        escaped: bool,
    }
    let mut runs = vec![];
    let mut slashes = 0;
    let mut offset = 0;
    while offset < line.len() {
        match line[offset] {
            b'\\' => {
                slashes += 1;
                offset += 1;
            }
            b'`' => {
                let escaped = slashes % 2 == 1;
                slashes = 0;
                let start = offset;
                while offset < line.len() && line[offset] == b'`' {
                    offset += 1;
                }
                runs.push(Run { start, end: offset, escaped });
            }
            _ => {
                slashes = 0;
                offset += 1;
            }
        }
    }
    let mut index = 0;
    while index < runs.len() {
        let run = &runs[index];
        if !run.escaped {
            let length = run.end - run.start;
            if let Some(close) = (index + 1..runs.len()).find(|&next| runs[next].end - runs[next].start == length) {
                inactive[run.start..runs[close].end].fill(true);
                index = close;
            }
        }
        index += 1;
    }
}

struct Angles {
    ends: Vec<i64>,
    nested: Vec<i64>,
}

fn angle_candidates(line: &[u8]) -> Angles {
    let n = line.len();
    let mut ends = [vec![-1i64; n + 1], vec![-1i64; n + 1], vec![-1i64; n + 1]];
    let mut nested = [vec![-1i64; n + 1], vec![-1i64; n + 1], vec![-1i64; n + 1]];
    for offset in (0..n).rev() {
        let byte = line[offset];
        if byte == b'>' {
            ends[0][offset] = offset as i64 + 1;
            nested[0][offset] = -1;
        } else {
            let state = match byte {
                b'\'' => 1,
                b'"' => 2,
                _ => 0,
            };
            ends[0][offset] = ends[state][offset + 1];
            nested[0][offset] = if byte == b'<' { offset as i64 } else { nested[state][offset + 1] };
        }
        let single = usize::from(byte != b'\'');
        ends[1][offset] = ends[single][offset + 1];
        nested[1][offset] = nested[single][offset + 1];
        let double = if byte == b'"' { 0 } else { 2 };
        ends[2][offset] = ends[double][offset + 1];
        nested[2][offset] = nested[double][offset + 1];
    }
    let [ends, _, _] = ends;
    let [nested, _, _] = nested;
    Angles { ends, nested }
}

fn is_inactive_angle(text: &[u8]) -> bool {
    let inner = &text[1..text.len() - 1];
    let no_space_angle = |b: &u8| !b.is_ascii_whitespace() && *b != b'<' && *b != b'>';
    if let Some(colon) = inner.iter().position(|b| *b == b':') {
        let scheme = &inner[..colon];
        if !scheme.is_empty()
            && scheme[0].is_ascii_alphabetic()
            && scheme[1..].iter().all(|b| b.is_ascii_alphanumeric() || b"+.-".contains(b))
            && inner[colon + 1..].iter().all(no_space_angle)
        {
            return true;
        }
    }
    if let Some(at) = inner.iter().position(|b| *b == b'@') {
        let ok = |part: &[u8]| !part.is_empty() && part.iter().all(|b| no_space_angle(b) && *b != b'@');
        if ok(&inner[..at]) && ok(&inner[at + 1..]) {
            return true;
        }
    }
    is_html_tag(text)
}

fn is_html_tag(text: &[u8]) -> bool {
    let peek = |at: usize| text.get(at).copied();
    let mut at = 1;
    if peek(at) == Some(b'/') {
        at += 1;
    }
    if !peek(at).is_some_and(|b| b.is_ascii_alphabetic()) {
        return false;
    }
    while peek(at).is_some_and(|b| b.is_ascii_alphanumeric() || b == b'-') {
        at += 1;
    }
    loop {
        let ws_start = at;
        while peek(at).is_some_and(|b| b.is_ascii_whitespace()) {
            at += 1;
        }
        let name_start = peek(at).is_some_and(|b| b.is_ascii_alphabetic() || b == b'_' || b == b':');
        if at == ws_start || !name_start {
            break;
        }
        while peek(at).is_some_and(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b)) {
            at += 1;
        }
        let save = at;
        while peek(at).is_some_and(|b| b.is_ascii_whitespace()) {
            at += 1;
        }
        if peek(at) != Some(b'=') {
            at = save;
            continue;
        }
        at += 1;
        while peek(at).is_some_and(|b| b.is_ascii_whitespace()) {
            at += 1;
        }
        match peek(at) {
            Some(quote @ (b'"' | b'\'')) => {
                at += 1;
                while peek(at).is_some_and(|b| b != quote) {
                    at += 1;
                }
                if peek(at) != Some(quote) {
                    return false;
                }
                at += 1;
            }
            Some(b) if !b.is_ascii_whitespace() && !b"\"'=<>`".contains(&b) => {
                while peek(at).is_some_and(|b| !b.is_ascii_whitespace() && !b"\"'=<>`".contains(&b)) {
                    at += 1;
                }
            }
            _ => return false,
        }
    }
    while peek(at).is_some_and(|b| b.is_ascii_whitespace()) {
        at += 1;
    }
    if peek(at) == Some(b'/') {
        at += 1;
    }
    peek(at) == Some(b'>') && at + 1 == text.len()
}

fn mark_angle_contexts(line: &[u8], inactive: &mut [bool], angles: &Angles) {
    let mut offset = 0;
    while offset < line.len() {
        if line[offset] != b'<' || inactive[offset] {
            offset += 1;
            continue;
        }
        let mut start = offset as i64;
        let mut end = angles.ends[offset + 1];
        let mut nested = angles.nested[offset + 1];
        while end > start && nested > start && nested < end {
            start = nested;
            end = angles.ends[start as usize + 1];
            nested = angles.nested[start as usize + 1];
        }
        offset = start as usize;
        if !inactive[offset] && end > start && is_inactive_angle(&line[offset..end as usize]) {
            inactive[offset..end as usize].fill(true);
            offset = end as usize - 1;
        }
        offset += 1;
    }
}

fn mark_explicit_destinations(line: &[u8], inactive: &mut [bool], angles: Option<&Angles>) {
    let n = line.len();
    let mut closings = vec![-1i64; n + 1];
    let mut whitespace = vec![-1i64; n + 1];
    let mut stack = vec![];
    let mut whitespace_start = 0;
    let mut offset = 0;
    while offset < n {
        match line[offset] {
            b'\\' => offset += 1,
            b'(' => stack.push(offset),
            b')' => {
                if let Some(open) = stack.pop() {
                    closings[open] = offset as i64;
                }
            }
            byte if is_ws(byte) => {
                whitespace[whitespace_start..=offset].fill(offset as i64);
                whitespace_start = offset + 1;
            }
            _ => {}
        }
        offset += 1;
    }
    let destination_end = |start: usize| -> usize {
        if line.get(start) == Some(&b'<') {
            let end = angles.map_or(-1, |angles| angles.ends[start + 1]);
            return if end > start as i64 { end as usize } else { start };
        }
        let closing = closings[start - 1];
        let ws = whitespace[start];
        let s = start as i64;
        if ws >= s && (closing < s || ws < closing) {
            return ws as usize;
        }
        if closing >= s { closing as usize } else { start }
    };
    let mut brackets = 0usize;
    let mut offset = 0;
    while offset < n {
        if inactive[offset] {
            offset += 1;
            continue;
        }
        match line[offset] {
            b'\\' => offset += 1,
            b'[' => brackets += 1,
            b']' => {
                if brackets > 0 && line.get(offset + 1) == Some(&b'(') {
                    let start = offset + 2;
                    let end = destination_end(start);
                    if end > start {
                        inactive[start..end].fill(true);
                    }
                }
                brackets = brackets.saturating_sub(1);
            }
            _ => {}
        }
        offset += 1;
    }
}

fn mark_bare_autolinks(line: &[u8], inactive: &mut [bool]) {
    let mut offset = 0;
    while offset < line.len() {
        let rest = &line[offset..];
        let starts = rest.starts_with(b"http://") || rest.starts_with(b"https://") || rest.starts_with(b"www.");
        let word_before = offset > 0 && (line[offset - 1].is_ascii_alphanumeric() || line[offset - 1] == b'_');
        if inactive[offset] || !starts || word_before {
            offset += 1;
            continue;
        }
        let mut end = offset;
        while end < line.len() && !is_ws(line[end]) && line[end] != b'<' && line[end] != b'>' {
            end += 1;
        }
        inactive[offset..end].fill(true);
        offset = end.max(offset + 1);
    }
}

fn bounds(line: &[u8]) -> (i64, i64) {
    let mut first = 0;
    while first < line.len() && is_ws(line[first]) {
        first += 1;
    }
    let mut last = line.len() as i64 - 1;
    while last >= first as i64 && is_ws(line[last as usize]) {
        last -= 1;
    }
    (first as i64, last)
}

fn pipes(line: &[u8], escaped: &[bool], hidden: &[usize]) -> Vec<usize> {
    (0..line.len()).filter(|&at| line[at] == b'|' && !escaped[at] && hidden.binary_search(&at).is_err()).collect()
}

fn cell_count(line: &[u8], escaped: &[bool], hidden: &[usize]) -> usize {
    let found = pipes(line, escaped, hidden);
    let (first, last) = bounds(line);
    let mut count = found.len() + 1;
    if found.first().map(|&p| p as i64) == Some(first) {
        count -= 1;
    }
    if found.last().map(|&p| p as i64) == Some(last) && found.last() != found.first() {
        count -= 1;
    }
    count
}
