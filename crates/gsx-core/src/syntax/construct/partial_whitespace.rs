use crate::syntax::event::{Event, Kind, Name};
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::{
    constant::HARD_BREAK_PREFIX_SIZE_MIN,
    slice::{Position, Slice},
};

pub fn resolve_whitespace(tokenizer: &mut Tokenizer, hard_break: bool, trim_whole: bool) {
    let mut index = 0;

    while index < tokenizer.events.len() {
        let event = &tokenizer.events[index];

        if event.kind == Kind::Exit && event.name == Name::Data && tokenizer.events[index - 1].link.is_none() {
            let trim_start =
                (trim_whole && index == 1) || (index > 1 && tokenizer.events[index - 2].name == Name::LineEnding);
            let trim_end = (trim_whole && index == tokenizer.events.len() - 1)
                || (index + 1 < tokenizer.events.len() && tokenizer.events[index + 1].name == Name::LineEnding);

            trim_data(tokenizer, index, trim_start, trim_end, hard_break);
        }

        index += 1;
    }

    tokenizer.map.consume(&mut tokenizer.events);
}

fn trim_data(tokenizer: &mut Tokenizer, exit_index: usize, trim_start: bool, trim_end: bool, hard_break: bool) {
    let mut slice =
        Slice::from_position(tokenizer.parse_state.bytes, &Position::from_exit_event(&tokenizer.events, exit_index));

    if trim_end {
        let mut index = slice.bytes.len();
        let mut tab = slice.after > 0;
        let mut spaces = 0;
        while index > 0 {
            match slice.bytes[index - 1] {
                b' ' => spaces += usize::from(!tab),
                b'\t' => tab = true,
                _ => break,
            }

            index -= 1;
        }

        let diff = slice.bytes.len() - index;
        let name = if hard_break && spaces >= HARD_BREAK_PREFIX_SIZE_MIN && exit_index + 1 < tokenizer.events.len() {
            Name::HardBreakTrailing
        } else {
            Name::SpaceOrTab
        };

        if index == 0 {
            tokenizer.events[exit_index - 1].name = name;
            tokenizer.events[exit_index].name = name;
            return;
        }

        if diff > 0 || slice.after > 0 {
            let exit_point = tokenizer.events[exit_index].point;
            let mut enter_point = exit_point;
            enter_point.index -= diff as u32;
            enter_point.column -= diff as u32;
            enter_point.vs = 0;

            tokenizer.map.add(
                exit_index + 1,
                0,
                vec![
                    Event { kind: Kind::Enter, name, point: enter_point, link: None },
                    Event { kind: Kind::Exit, name, point: exit_point, link: None },
                ],
            );

            tokenizer.events[exit_index].point = enter_point;
            slice.bytes = &slice.bytes[..index];
        }
    }

    if trim_start {
        let mut index = 0;
        while index < slice.bytes.len() {
            match slice.bytes[index] {
                b' ' | b'\t' => index += 1,
                _ => break,
            }
        }

        if index == slice.bytes.len() {
            tokenizer.events[exit_index - 1].name = Name::SpaceOrTab;
            tokenizer.events[exit_index].name = Name::SpaceOrTab;
            return;
        }

        if index > 0 || slice.before > 0 {
            let enter_point = tokenizer.events[exit_index - 1].point;
            let mut exit_point = enter_point;
            let skip = index + usize::from(slice.before > 0);
            exit_point.index += skip as u32;
            exit_point.column += skip as u32;
            exit_point.vs = 0;

            tokenizer.map.add(
                exit_index - 1,
                0,
                vec![
                    Event { kind: Kind::Enter, name: Name::SpaceOrTab, point: enter_point, link: None },
                    Event { kind: Kind::Exit, name: Name::SpaceOrTab, point: exit_point, link: None },
                ],
            );

            tokenizer.events[exit_index - 1].point = exit_point;
        }
    }
}
