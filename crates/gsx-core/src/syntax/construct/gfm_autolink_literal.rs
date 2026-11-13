use crate::syntax::event::{Event, Kind, Name};
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::{
    char::{Kind as CharacterKind, kind_after_index},
    slice::{Position, Slice},
};

pub fn protocol_start(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'H' | b'h'))
        && !matches!(tokenizer.previous, Some(b'A'..=b'Z' | b'a'..=b'z'))
        && tokenizer.tokenize_state.label_starts.is_empty()
    {
        tokenizer.enter(Name::GfmAutolinkLiteralProtocol);
        tokenizer.attempt(State::Next(StateName::GfmAutolinkLiteralProtocolAfter), State::Nok);
        tokenizer.attempt(State::Next(StateName::GfmAutolinkLiteralDomainInside), State::Nok);
        tokenizer.tokenize_state.start = tokenizer.point.index;
        State::Retry(StateName::GfmAutolinkLiteralProtocolPrefixInside)
    } else {
        State::Nok
    }
}

pub fn protocol_after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.exit(Name::GfmAutolinkLiteralProtocol);
    State::Ok
}

pub fn protocol_prefix_inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'A'..=b'Z' | b'a'..=b'z') if tokenizer.point.index - tokenizer.tokenize_state.start < 5 => {
            tokenizer.consume();
            State::Next(StateName::GfmAutolinkLiteralProtocolPrefixInside)
        }
        Some(b':') => {
            let slice =
                Slice::from_indices(tokenizer.parse_state.bytes, tokenizer.tokenize_state.start, tokenizer.point.index);
            let name = slice.as_str().to_ascii_lowercase();

            tokenizer.tokenize_state.start = 0;

            if name == "http" || name == "https" {
                tokenizer.consume();
                State::Next(StateName::GfmAutolinkLiteralProtocolSlashesInside)
            } else {
                State::Nok
            }
        }
        _ => {
            tokenizer.tokenize_state.start = 0;
            State::Nok
        }
    }
}

pub fn protocol_slashes_inside(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'/') {
        tokenizer.consume();
        if tokenizer.tokenize_state.size == 0 {
            tokenizer.tokenize_state.size += 1;
            State::Next(StateName::GfmAutolinkLiteralProtocolSlashesInside)
        } else {
            tokenizer.tokenize_state.size = 0;
            State::Ok
        }
    } else {
        tokenizer.tokenize_state.size = 0;
        State::Nok
    }
}

pub fn www_start(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'W' | b'w'))
        && matches!(tokenizer.previous, None | Some(b'\t' | b'\n' | b' ' | b'(' | b'*' | b'_' | b'[' | b']' | b'~'))
        && tokenizer.tokenize_state.label_starts.is_empty()
    {
        tokenizer.enter(Name::GfmAutolinkLiteralWww);
        tokenizer.attempt(State::Next(StateName::GfmAutolinkLiteralWwwAfter), State::Nok);
        tokenizer.check(State::Next(StateName::GfmAutolinkLiteralDomainInside), State::Nok);
        State::Retry(StateName::GfmAutolinkLiteralWwwPrefixInside)
    } else {
        State::Nok
    }
}

pub fn www_after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.exit(Name::GfmAutolinkLiteralWww);
    State::Ok
}

pub fn www_prefix_inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'.') if tokenizer.tokenize_state.size == 3 => {
            tokenizer.tokenize_state.size = 0;
            tokenizer.consume();
            State::Next(StateName::GfmAutolinkLiteralWwwPrefixAfter)
        }
        Some(b'W' | b'w') if tokenizer.tokenize_state.size < 3 => {
            tokenizer.tokenize_state.size += 1;
            tokenizer.consume();
            State::Next(StateName::GfmAutolinkLiteralWwwPrefixInside)
        }
        _ => {
            tokenizer.tokenize_state.size = 0;
            State::Nok
        }
    }
}

pub fn www_prefix_after(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current.is_none() { State::Nok } else { State::Ok }
}

pub fn domain_inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'.' | b'_') => {
            tokenizer.check(
                State::Next(StateName::GfmAutolinkLiteralDomainAfter),
                State::Next(StateName::GfmAutolinkLiteralDomainAtPunctuation),
            );
            trail_start(tokenizer)
        }
        Some(b'-' | 0x80..=0xBF) => {
            tokenizer.consume();
            State::Next(StateName::GfmAutolinkLiteralDomainInside)
        }
        _ => {
            if tokenizer.current.is_some()
                && kind_after_index(tokenizer.parse_state.bytes, tokenizer.point.index) == CharacterKind::Other
            {
                tokenizer.tokenize_state.seen = true;
                tokenizer.consume();
                State::Next(StateName::GfmAutolinkLiteralDomainInside)
            } else {
                State::Retry(StateName::GfmAutolinkLiteralDomainAfter)
            }
        }
    }
}

pub fn domain_at_punctuation(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'_')) {
        tokenizer.tokenize_state.marker = b'_';
    } else {
        tokenizer.tokenize_state.marker_b = tokenizer.tokenize_state.marker;
        tokenizer.tokenize_state.marker = 0;
    }

    tokenizer.consume();
    State::Next(StateName::GfmAutolinkLiteralDomainInside)
}

pub fn domain_after(tokenizer: &mut Tokenizer) -> State {
    let result = if tokenizer.tokenize_state.marker_b == b'_'
        || tokenizer.tokenize_state.marker == b'_'
        || !tokenizer.tokenize_state.seen
    {
        State::Nok
    } else {
        State::Retry(StateName::GfmAutolinkLiteralPathInside)
    };

    tokenizer.tokenize_state.seen = false;
    tokenizer.tokenize_state.marker = 0;
    tokenizer.tokenize_state.marker_b = 0;
    result
}

pub fn path_inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(0x80..=0xBF) => {
            tokenizer.consume();
            State::Next(StateName::GfmAutolinkLiteralPathInside)
        }
        Some(b'(') => {
            tokenizer.tokenize_state.size += 1;
            tokenizer.consume();
            State::Next(StateName::GfmAutolinkLiteralPathInside)
        }
        Some(
            b'!' | b'"' | b'&' | b'\'' | b')' | b'*' | b',' | b'.' | b':' | b';' | b'<' | b'?' | b']' | b'_' | b'~',
        ) => {
            let next =
                if tokenizer.current == Some(b')') && tokenizer.tokenize_state.size_b < tokenizer.tokenize_state.size {
                    StateName::GfmAutolinkLiteralPathAtPunctuation
                } else {
                    StateName::GfmAutolinkLiteralPathAfter
                };
            tokenizer.check(State::Next(next), State::Next(StateName::GfmAutolinkLiteralPathAtPunctuation));
            trail_start(tokenizer)
        }
        _ => {
            if tokenizer.current.is_none()
                || kind_after_index(tokenizer.parse_state.bytes, tokenizer.point.index) == CharacterKind::Whitespace
            {
                State::Retry(StateName::GfmAutolinkLiteralPathAfter)
            } else {
                tokenizer.consume();
                State::Next(StateName::GfmAutolinkLiteralPathInside)
            }
        }
    }
}

pub fn path_at_punctuation(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b')') {
        tokenizer.tokenize_state.size_b += 1;
    }

    tokenizer.consume();
    State::Next(StateName::GfmAutolinkLiteralPathInside)
}

pub fn path_after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.size = 0;
    tokenizer.tokenize_state.size_b = 0;
    State::Ok
}

fn trail_start(tokenizer: &mut Tokenizer) -> State {
    let index = tokenizer.point.index;
    let (start, end, ok) = tokenizer.tokenize_state.autolink_trail;
    if (start..end).contains(&index) {
        return if ok { State::Ok } else { State::Nok };
    }
    tokenizer.tokenize_state.autolink_trail = (index, index, false);
    State::Retry(StateName::GfmAutolinkLiteralTrail)
}

fn trail_end(tokenizer: &mut Tokenizer, ok: bool) -> State {
    let trail = &mut tokenizer.tokenize_state.autolink_trail;
    (trail.1, trail.2) = (tokenizer.point.index, ok);
    if ok { State::Ok } else { State::Nok }
}

pub fn trail(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'!' | b'"' | b'\'' | b')' | b'*' | b',' | b'.' | b':' | b';' | b'?' | b'_' | b'~') => {
            tokenizer.consume();
            State::Next(StateName::GfmAutolinkLiteralTrail)
        }
        Some(b'&') => {
            tokenizer.consume();
            State::Next(StateName::GfmAutolinkLiteralTrailCharRefStart)
        }
        Some(b'<') => trail_end(tokenizer, true),
        Some(b']') => {
            tokenizer.consume();
            State::Next(StateName::GfmAutolinkLiteralTrailBracketAfter)
        }
        _ => {
            let ok = tokenizer.current.is_none()
                || kind_after_index(tokenizer.parse_state.bytes, tokenizer.point.index) == CharacterKind::Whitespace;
            trail_end(tokenizer, ok)
        }
    }
}

pub fn trail_bracket_after(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, None | Some(b'\t' | b'\n' | b' ' | b'(' | b'[')) {
        trail_end(tokenizer, true)
    } else {
        State::Retry(StateName::GfmAutolinkLiteralTrail)
    }
}

pub fn trail_char_ref_start(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'A'..=b'Z' | b'a'..=b'z')) {
        State::Retry(StateName::GfmAutolinkLiteralTrailCharRefInside)
    } else {
        trail_end(tokenizer, false)
    }
}

pub fn trail_char_ref_inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'A'..=b'Z' | b'a'..=b'z') => {
            tokenizer.consume();
            State::Next(StateName::GfmAutolinkLiteralTrailCharRefInside)
        }
        Some(b';') => {
            tokenizer.consume();
            State::Next(StateName::GfmAutolinkLiteralTrail)
        }
        _ => trail_end(tokenizer, false),
    }
}

pub fn resolve(tokenizer: &mut Tokenizer) {
    tokenizer.map.consume(&mut tokenizer.events);

    let mut index = 0;
    let mut links = 0;

    while index < tokenizer.events.len() {
        let event = &tokenizer.events[index];

        if event.kind == Kind::Enter {
            if event.name == Name::Link {
                links += 1;
            }
        } else {
            if event.name == Name::Data && links == 0 && tokenizer.events[index - 1].link.is_none() {
                let slice = Slice::from_position(
                    tokenizer.parse_state.bytes,
                    &Position::from_exit_event(&tokenizer.events, index),
                );
                let bytes = slice.bytes;
                let mut byte_index = 0;
                let mut replace = Vec::new();
                let mut point = tokenizer.events[index - 1].point;
                let start_index = point.index;
                let mut min = 0;

                while byte_index < bytes.len() {
                    if bytes[byte_index] == b'@' {
                        let mut range = (0, 0, Name::GfmAutolinkLiteralEmail);

                        if let Some(start) = peek_bytes_atext(bytes, min, byte_index)
                            && !email_previous(tokenizer.parse_state.bytes, start_index + start)
                            && let Some(end) = peek_bytes_email_domain(bytes, byte_index + 1)
                        {
                            range = (start, end, Name::GfmAutolinkLiteralEmail);
                        }

                        if range.1 != 0 {
                            byte_index = range.1;

                            if min != range.0 {
                                replace.push(Event { kind: Kind::Enter, name: Name::Data, point, link: None });
                                point = point.shift_to(tokenizer.parse_state.bytes, start_index + range.0);
                                replace.push(Event { kind: Kind::Exit, name: Name::Data, point, link: None });
                            }

                            replace.push(Event { kind: Kind::Enter, name: range.2, point, link: None });
                            point = point.shift_to(tokenizer.parse_state.bytes, start_index + range.1);
                            replace.push(Event { kind: Kind::Exit, name: range.2, point, link: None });
                            min = range.1;
                        }
                    }

                    byte_index += 1;
                }

                if min != 0 && min < bytes.len() {
                    replace.push(Event { kind: Kind::Enter, name: Name::Data, point, link: None });
                    replace.push(Event { kind: Kind::Exit, name: Name::Data, point: event.point, link: None });
                }

                if !replace.is_empty() {
                    tokenizer.map.add(index - 1, 2, replace);
                }
            }

            if event.name == Name::Link {
                links -= 1;
            }
        }

        index += 1;
    }
}

fn peek_bytes_atext(bytes: &[u8], min: usize, end: usize) -> Option<usize> {
    let mut index = end;

    while index > min && matches!(bytes[index - 1], b'+' | b'-' | b'.' | b'0'..=b'9' | b'A'..=b'Z' | b'_' | b'a'..=b'z')
    {
        index -= 1;
    }

    if index == end || (index > min && bytes[index - 1] == b'/') { None } else { Some(index) }
}

fn email_previous(bytes: &[u8], index: usize) -> bool {
    index > 0 && matches!(bytes[index - 1], b'/' | b'+' | b'-' | b'.' | b'0'..=b'9' | b'A'..=b'Z' | b'_' | b'a'..=b'z')
}

fn peek_bytes_email_domain(bytes: &[u8], start: usize) -> Option<usize> {
    let mut index = start;
    let mut dot = false;

    while index < bytes.len() {
        match bytes[index] {
            b'-' | b'0'..=b'9' | b'A'..=b'Z' | b'_' | b'a'..=b'z' => {}
            b'.' if index + 1 < bytes.len() && matches!(bytes[index + 1], b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z') => {
                dot = true;
            }
            _ => break,
        }

        index += 1;
    }

    if index > start && dot && matches!(bytes[index - 1], b'.' | b'A'..=b'Z' | b'a'..=b'z') {
        Some(index)
    } else {
        None
    }
}
