use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::constant::{AUTOLINK_DOMAIN_SIZE_MAX, AUTOLINK_SCHEME_SIZE_MAX};

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'<') {
        tokenizer.enter(Name::Autolink);
        tokenizer.enter(Name::AutolinkMarker);
        tokenizer.consume();
        tokenizer.exit(Name::AutolinkMarker);
        tokenizer.enter(Name::AutolinkProtocol);
        State::Next(StateName::AutolinkOpen)
    } else {
        State::Nok
    }
}

pub fn open(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'A'..=b'Z' | b'a'..=b'z') => {
            tokenizer.consume();
            State::Next(StateName::AutolinkSchemeOrEmailAtext)
        }
        Some(b'@') => State::Nok,
        _ => State::Retry(StateName::AutolinkEmailAtext),
    }
}

pub fn scheme_or_email_atext(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'+' | b'-' | b'.' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z') => {
            tokenizer.tokenize_state.size = 1;
            State::Retry(StateName::AutolinkSchemeInsideOrEmailAtext)
        }
        _ => State::Retry(StateName::AutolinkEmailAtext),
    }
}

pub fn scheme_inside_or_email_atext(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b':') => {
            tokenizer.consume();
            tokenizer.tokenize_state.size = 0;
            State::Next(StateName::AutolinkUrlInside)
        }
        Some(b'+' | b'-' | b'.' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z')
            if tokenizer.tokenize_state.size < AUTOLINK_SCHEME_SIZE_MAX =>
        {
            tokenizer.consume();
            tokenizer.tokenize_state.size += 1;
            State::Next(StateName::AutolinkSchemeInsideOrEmailAtext)
        }
        _ => {
            tokenizer.tokenize_state.size = 0;
            State::Retry(StateName::AutolinkEmailAtext)
        }
    }
}

pub fn url_inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'>') => {
            tokenizer.exit(Name::AutolinkProtocol);
            tokenizer.enter(Name::AutolinkMarker);
            tokenizer.consume();
            tokenizer.exit(Name::AutolinkMarker);
            tokenizer.exit(Name::Autolink);
            State::Ok
        }
        None | Some(b'\0'..=0x1F | b' ' | b'<' | 0x7F) => State::Nok,
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::AutolinkUrlInside)
        }
    }
}

pub fn email_atext(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'@') => {
            tokenizer.consume();
            State::Next(StateName::AutolinkEmailAtSignOrDot)
        }
        Some(b'#'..=b'\'' | b'*' | b'+' | b'-'..=b'9' | b'=' | b'?' | b'A'..=b'Z' | b'^'..=b'~') => {
            tokenizer.consume();
            State::Next(StateName::AutolinkEmailAtext)
        }
        _ => State::Nok,
    }
}

pub fn email_at_sign_or_dot(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z') => State::Retry(StateName::AutolinkEmailValue),
        _ => State::Nok,
    }
}

pub fn email_label(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'.') => {
            tokenizer.consume();
            tokenizer.tokenize_state.size = 0;
            State::Next(StateName::AutolinkEmailAtSignOrDot)
        }
        Some(b'>') => {
            let index = tokenizer.events.len();
            tokenizer.exit(Name::AutolinkProtocol);
            tokenizer.events[index - 1].name = Name::AutolinkEmail;
            tokenizer.events[index].name = Name::AutolinkEmail;
            tokenizer.enter(Name::AutolinkMarker);
            tokenizer.consume();
            tokenizer.exit(Name::AutolinkMarker);
            tokenizer.exit(Name::Autolink);
            tokenizer.tokenize_state.size = 0;
            State::Ok
        }
        _ => State::Retry(StateName::AutolinkEmailValue),
    }
}

pub fn email_value(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'-' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z')
            if tokenizer.tokenize_state.size < AUTOLINK_DOMAIN_SIZE_MAX =>
        {
            let name = if matches!(tokenizer.current, Some(b'-')) {
                StateName::AutolinkEmailValue
            } else {
                StateName::AutolinkEmailLabel
            };
            tokenizer.tokenize_state.size += 1;
            tokenizer.consume();
            State::Next(name)
        }
        _ => {
            tokenizer.tokenize_state.size = 0;
            State::Nok
        }
    }
}
