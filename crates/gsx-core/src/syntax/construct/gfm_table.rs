use crate::syntax::construct::partial_space_or_tab::{space_or_tab, space_or_tab_min_max};
use crate::syntax::event::{Content, Event, Kind, Link, Name};
use crate::syntax::resolve::Name as ResolveName;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::Subresult;
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::{constant::TAB_SIZE, skip::opt_back as skip_opt_back};
mod spoiler;

fn current(tokenizer: &Tokenizer) -> Option<u8> {
    match tokenizer.current {
        Some(b'|') if tokenizer.tokenize_state.gfm_table_pipe_data.binary_search(&tokenizer.point.index).is_ok() => {
            Some(b'A')
        }
        other => other,
    }
}

fn line_end(bytes: &[u8], start: usize) -> usize {
    bytes[start..].iter().position(|&byte| byte == b'\n').map_or(bytes.len(), |end| start + end)
}

fn set_pipe_data(tokenizer: &mut Tokenizer, columns: Option<usize>, exact: bool) {
    let bytes = tokenizer.parse_state.bytes;
    let start = tokenizer.point.index;
    let end = line_end(bytes, start);
    let data = columns.map_or_else(Vec::new, |columns| spoiler::data_pipes(&bytes[start..end], columns, exact));
    tokenizer.tokenize_state.gfm_table_pipe_data = data.into_iter().map(|offset| start + offset).collect();
}

fn next_delimiter_columns(bytes: &[u8], start: usize) -> Option<usize> {
    let next = line_end(bytes, start) + 1;
    if next > bytes.len() {
        return None;
    }
    let line = &bytes[next..line_end(bytes, next)];
    let content = line.iter().position(|byte| !matches!(byte, b' ' | b'\t' | b'>')).map_or(&[][..], |at| &line[at..]);
    spoiler::delimiter_columns(content)
}

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if !tokenizer.pierce
        && !tokenizer.events.is_empty()
        && matches!(
            tokenizer.events
                [skip_opt_back(&tokenizer.events, tokenizer.events.len() - 1, &[Name::LineEnding, Name::SpaceOrTab],)]
            .name,
            Name::GfmTableHead | Name::GfmTableRow
        )
    {
        State::Retry(StateName::GfmTableBodyRowStart)
    } else {
        State::Retry(StateName::GfmTableHeadRowBefore)
    }
}

pub fn head_row_before(tokenizer: &mut Tokenizer) -> State {
    let bytes = tokenizer.parse_state.bytes;
    let next = line_end(bytes, tokenizer.point.index) + 1;
    if next > bytes.len() || !bytes[next..line_end(bytes, next)].contains(&b'-') {
        return State::Nok;
    }
    let columns = next_delimiter_columns(bytes, tokenizer.point.index);
    set_pipe_data(tokenizer, columns, true);
    tokenizer.enter(Name::GfmTableHead);
    tokenizer.enter(Name::GfmTableRow);
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(State::Next(StateName::GfmTableHeadRowStart), State::Nok);
        State::Retry(space_or_tab_min_max(tokenizer, 0, TAB_SIZE - 1))
    } else {
        State::Retry(StateName::GfmTableHeadRowStart)
    }
}

pub fn head_row_start(tokenizer: &mut Tokenizer) -> State {
    match current(tokenizer) {
        Some(b'\t' | b' ') => State::Nok,
        Some(b'|') => State::Retry(StateName::GfmTableHeadRowBreak),
        _ => {
            tokenizer.tokenize_state.seen = true;
            tokenizer.tokenize_state.size_b += 1;
            State::Retry(StateName::GfmTableHeadRowBreak)
        }
    }
}

pub fn head_row_break(tokenizer: &mut Tokenizer) -> State {
    match current(tokenizer) {
        None => {
            tokenizer.tokenize_state.seen = false;
            tokenizer.tokenize_state.size = 0;
            tokenizer.tokenize_state.size_b = 0;
            State::Nok
        }
        Some(b'\n') => {
            if tokenizer.tokenize_state.size_b > 1 {
                tokenizer.tokenize_state.size_b = 0;
                tokenizer.interrupt = true;
                tokenizer.exit(Name::GfmTableRow);
                tokenizer.enter(Name::LineEnding);
                tokenizer.consume();
                tokenizer.exit(Name::LineEnding);
                State::Next(StateName::GfmTableHeadDelimiterStart)
            } else {
                tokenizer.tokenize_state.seen = false;
                tokenizer.tokenize_state.size = 0;
                tokenizer.tokenize_state.size_b = 0;
                State::Nok
            }
        }
        Some(b'\t' | b' ') => {
            tokenizer.attempt(State::Next(StateName::GfmTableHeadRowBreak), State::Nok);
            State::Retry(space_or_tab(tokenizer))
        }
        _ => {
            tokenizer.tokenize_state.size_b += 1;

            if tokenizer.tokenize_state.seen {
                tokenizer.tokenize_state.seen = false;
                tokenizer.tokenize_state.size += 1;
            }

            if current(tokenizer) == Some(b'|') {
                tokenizer.enter(Name::GfmTableCellDivider);
                tokenizer.consume();
                tokenizer.exit(Name::GfmTableCellDivider);
                tokenizer.tokenize_state.seen = true;
                State::Next(StateName::GfmTableHeadRowBreak)
            } else {
                tokenizer.enter(Name::Data);
                State::Retry(StateName::GfmTableHeadRowData)
            }
        }
    }
}

pub fn head_row_data(tokenizer: &mut Tokenizer) -> State {
    match current(tokenizer) {
        None | Some(b'\t' | b'\n' | b' ' | b'|') => {
            tokenizer.exit(Name::Data);
            State::Retry(StateName::GfmTableHeadRowBreak)
        }
        _ => {
            let name = if tokenizer.current == Some(b'\\') {
                StateName::GfmTableHeadRowEscape
            } else {
                StateName::GfmTableHeadRowData
            };
            tokenizer.consume();
            State::Next(name)
        }
    }
}

pub fn head_row_escape(tokenizer: &mut Tokenizer) -> State {
    match current(tokenizer) {
        Some(b'\\' | b'|') => {
            tokenizer.consume();
            State::Next(StateName::GfmTableHeadRowData)
        }
        _ => State::Retry(StateName::GfmTableHeadRowData),
    }
}

pub fn head_delimiter_start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.interrupt = false;

    if tokenizer.lazy || tokenizer.pierce {
        tokenizer.tokenize_state.size = 0;
        tokenizer.tokenize_state.seen = false;
        State::Nok
    } else {
        tokenizer.enter(Name::GfmTableDelimiterRow);
        tokenizer.tokenize_state.seen = false;

        match current(tokenizer) {
            Some(b'\t' | b' ') => {
                tokenizer.attempt(
                    State::Next(StateName::GfmTableHeadDelimiterBefore),
                    State::Next(StateName::GfmTableHeadDelimiterNok),
                );

                State::Retry(space_or_tab_min_max(tokenizer, 0, TAB_SIZE - 1))
            }
            _ => State::Retry(StateName::GfmTableHeadDelimiterBefore),
        }
    }
}

pub fn head_delimiter_before(tokenizer: &mut Tokenizer) -> State {
    match current(tokenizer) {
        Some(b'-' | b':') => State::Retry(StateName::GfmTableHeadDelimiterValueBefore),
        Some(b'|') => {
            tokenizer.tokenize_state.seen = true;
            tokenizer.enter(Name::GfmTableCellDivider);
            tokenizer.consume();
            tokenizer.exit(Name::GfmTableCellDivider);
            State::Next(StateName::GfmTableHeadDelimiterCellBefore)
        }
        _ => State::Retry(StateName::GfmTableHeadDelimiterNok),
    }
}

pub fn head_delimiter_cell_before(tokenizer: &mut Tokenizer) -> State {
    match current(tokenizer) {
        Some(b'\t' | b' ') => {
            tokenizer.attempt(State::Next(StateName::GfmTableHeadDelimiterValueBefore), State::Nok);
            State::Retry(space_or_tab(tokenizer))
        }
        _ => State::Retry(StateName::GfmTableHeadDelimiterValueBefore),
    }
}

pub fn head_delimiter_value_before(tokenizer: &mut Tokenizer) -> State {
    match current(tokenizer) {
        None | Some(b'\n') => State::Retry(StateName::GfmTableHeadDelimiterCellAfter),
        Some(b':') => {
            tokenizer.tokenize_state.size_b += 1;
            tokenizer.tokenize_state.seen = true;
            tokenizer.enter(Name::GfmTableDelimiterMarker);
            tokenizer.consume();
            tokenizer.exit(Name::GfmTableDelimiterMarker);
            State::Next(StateName::GfmTableHeadDelimiterLeftAlignmentAfter)
        }
        Some(b'-') => {
            tokenizer.tokenize_state.size_b += 1;
            State::Retry(StateName::GfmTableHeadDelimiterLeftAlignmentAfter)
        }
        _ => State::Retry(StateName::GfmTableHeadDelimiterNok),
    }
}

pub fn head_delimiter_left_alignment_after(tokenizer: &mut Tokenizer) -> State {
    match current(tokenizer) {
        Some(b'-') => {
            tokenizer.enter(Name::GfmTableDelimiterFiller);
            State::Retry(StateName::GfmTableHeadDelimiterFiller)
        }
        _ => State::Retry(StateName::GfmTableHeadDelimiterNok),
    }
}

pub fn head_delimiter_filler(tokenizer: &mut Tokenizer) -> State {
    match current(tokenizer) {
        Some(b'-') => {
            tokenizer.consume();
            State::Next(StateName::GfmTableHeadDelimiterFiller)
        }
        Some(b':') => {
            tokenizer.tokenize_state.seen = true;
            tokenizer.exit(Name::GfmTableDelimiterFiller);
            tokenizer.enter(Name::GfmTableDelimiterMarker);
            tokenizer.consume();
            tokenizer.exit(Name::GfmTableDelimiterMarker);
            State::Next(StateName::GfmTableHeadDelimiterRightAlignmentAfter)
        }
        _ => {
            tokenizer.exit(Name::GfmTableDelimiterFiller);
            State::Retry(StateName::GfmTableHeadDelimiterRightAlignmentAfter)
        }
    }
}

pub fn head_delimiter_right_alignment_after(tokenizer: &mut Tokenizer) -> State {
    match current(tokenizer) {
        Some(b'\t' | b' ') => {
            tokenizer.attempt(State::Next(StateName::GfmTableHeadDelimiterCellAfter), State::Nok);
            State::Retry(space_or_tab(tokenizer))
        }
        _ => State::Retry(StateName::GfmTableHeadDelimiterCellAfter),
    }
}

pub fn head_delimiter_cell_after(tokenizer: &mut Tokenizer) -> State {
    match current(tokenizer) {
        None | Some(b'\n') => {
            if !tokenizer.tokenize_state.seen || tokenizer.tokenize_state.size != tokenizer.tokenize_state.size_b {
                State::Retry(StateName::GfmTableHeadDelimiterNok)
            } else {
                tokenizer.tokenize_state.gfm_table_columns = tokenizer.tokenize_state.size_b;
                tokenizer.tokenize_state.seen = false;
                tokenizer.tokenize_state.size = 0;
                tokenizer.tokenize_state.size_b = 0;
                tokenizer.exit(Name::GfmTableDelimiterRow);
                tokenizer.exit(Name::GfmTableHead);
                tokenizer.register_resolver(ResolveName::GfmTable);
                State::Ok
            }
        }
        Some(b'|') => State::Retry(StateName::GfmTableHeadDelimiterBefore),
        _ => State::Retry(StateName::GfmTableHeadDelimiterNok),
    }
}

pub fn head_delimiter_nok(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.seen = false;
    tokenizer.tokenize_state.size = 0;
    tokenizer.tokenize_state.size_b = 0;
    State::Nok
}

pub fn body_row_start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.lazy {
        State::Nok
    } else {
        set_pipe_data(tokenizer, Some(tokenizer.tokenize_state.gfm_table_columns), false);
        tokenizer.enter(Name::GfmTableRow);

        match current(tokenizer) {
            Some(b'\t' | b' ') => {
                tokenizer.attempt(State::Next(StateName::GfmTableBodyRowBreak), State::Nok);
                State::Retry(space_or_tab_min_max(tokenizer, 0, usize::MAX))
            }
            _ => State::Retry(StateName::GfmTableBodyRowBreak),
        }
    }
}

pub fn body_row_break(tokenizer: &mut Tokenizer) -> State {
    match current(tokenizer) {
        None | Some(b'\n') => {
            tokenizer.exit(Name::GfmTableRow);
            State::Ok
        }
        Some(b'\t' | b' ') => {
            tokenizer.attempt(State::Next(StateName::GfmTableBodyRowBreak), State::Nok);
            State::Retry(space_or_tab(tokenizer))
        }
        Some(b'|') => {
            tokenizer.enter(Name::GfmTableCellDivider);
            tokenizer.consume();
            tokenizer.exit(Name::GfmTableCellDivider);
            State::Next(StateName::GfmTableBodyRowBreak)
        }
        _ => {
            tokenizer.enter(Name::Data);
            State::Retry(StateName::GfmTableBodyRowData)
        }
    }
}

pub fn body_row_data(tokenizer: &mut Tokenizer) -> State {
    match current(tokenizer) {
        None | Some(b'\t' | b'\n' | b' ' | b'|') => {
            tokenizer.exit(Name::Data);
            State::Retry(StateName::GfmTableBodyRowBreak)
        }
        _ => {
            let name = if tokenizer.current == Some(b'\\') {
                StateName::GfmTableBodyRowEscape
            } else {
                StateName::GfmTableBodyRowData
            };
            tokenizer.consume();
            State::Next(name)
        }
    }
}

pub fn body_row_escape(tokenizer: &mut Tokenizer) -> State {
    match current(tokenizer) {
        Some(b'\\' | b'|') => {
            tokenizer.consume();
            State::Next(StateName::GfmTableBodyRowData)
        }
        _ => State::Retry(StateName::GfmTableBodyRowData),
    }
}

pub fn resolve(tokenizer: &mut Tokenizer) -> Option<Subresult> {
    let mut index = 0;
    let mut in_first_cell_awaiting_pipe = true;
    let mut in_row = false;
    let mut in_delimiter_row = false;
    let mut last_cell = (0, 0, 0, 0);
    let mut cell = (0, 0, 0, 0);
    let mut after_head_awaiting_first_body_row = false;
    let mut last_table_end = 0;
    let mut last_table_has_body = false;

    while index < tokenizer.events.len() {
        let event = &tokenizer.events[index];

        if event.kind == Kind::Enter {
            if event.name == Name::GfmTableHead {
                after_head_awaiting_first_body_row = false;

                if last_table_end != 0 {
                    flush_table_end(tokenizer, last_table_end, last_table_has_body);
                    last_table_has_body = false;
                    last_table_end = 0;
                }

                let enter =
                    Event { kind: Kind::Enter, name: Name::GfmTable, point: tokenizer.events[index].point, link: None };
                tokenizer.map.add(index, 0, vec![enter]);
            } else if matches!(event.name, Name::GfmTableRow | Name::GfmTableDelimiterRow) {
                in_delimiter_row = event.name == Name::GfmTableDelimiterRow;
                in_row = true;
                in_first_cell_awaiting_pipe = true;
                last_cell = (0, 0, 0, 0);
                cell = (0, index + 1, 0, 0);

                if after_head_awaiting_first_body_row {
                    after_head_awaiting_first_body_row = false;
                    last_table_has_body = true;
                    let enter = Event {
                        kind: Kind::Enter,
                        name: Name::GfmTableBody,
                        point: tokenizer.events[index].point,
                        link: None,
                    };
                    tokenizer.map.add(index, 0, vec![enter]);
                }
            } else if in_row
                && matches!(event.name, Name::Data | Name::GfmTableDelimiterMarker | Name::GfmTableDelimiterFiller)
            {
                in_first_cell_awaiting_pipe = false;

                if cell.2 == 0 {
                    if last_cell.1 != 0 {
                        cell.0 = cell.1;
                        flush_cell(tokenizer, last_cell, in_delimiter_row, None);
                        last_cell = (0, 0, 0, 0);
                    }

                    cell.2 = index;
                }
            } else if event.name == Name::GfmTableCellDivider {
                if in_first_cell_awaiting_pipe {
                    in_first_cell_awaiting_pipe = false;
                } else {
                    if last_cell.1 != 0 {
                        cell.0 = cell.1;
                        flush_cell(tokenizer, last_cell, in_delimiter_row, None);
                    }

                    last_cell = cell;
                    cell = (last_cell.1, index, 0, 0);
                }
            }
        } else if event.name == Name::GfmTableHead {
            after_head_awaiting_first_body_row = true;
            last_table_end = index;
        } else if matches!(event.name, Name::GfmTableRow | Name::GfmTableDelimiterRow) {
            in_row = false;
            last_table_end = index;
            if last_cell.1 != 0 {
                cell.0 = cell.1;
                flush_cell(tokenizer, last_cell, in_delimiter_row, Some(index));
            } else if cell.1 != 0 {
                flush_cell(tokenizer, cell, in_delimiter_row, Some(index));
            }
        } else if in_row
            && (matches!(event.name, Name::Data | Name::GfmTableDelimiterMarker | Name::GfmTableDelimiterFiller))
        {
            cell.3 = index;
        }

        index += 1;
    }

    if last_table_end != 0 {
        flush_table_end(tokenizer, last_table_end, last_table_has_body);
    }

    tokenizer.map.consume(&mut tokenizer.events);
    None
}

fn flush_cell(
    tokenizer: &mut Tokenizer,
    range: (usize, usize, usize, usize),
    in_delimiter_row: bool,
    row_end: Option<usize>,
) {
    let group_name = if in_delimiter_row { Name::GfmTableDelimiterCell } else { Name::GfmTableCell };
    let value_name = if in_delimiter_row { Name::GfmTableDelimiterCellValue } else { Name::GfmTableCellText };

    if range.0 != 0 {
        tokenizer.map.add(
            range.0,
            0,
            vec![Event { kind: Kind::Exit, name: group_name, point: tokenizer.events[range.0].point, link: None }],
        );
    }

    tokenizer.map.add(
        range.1,
        0,
        vec![Event { kind: Kind::Enter, name: group_name, point: tokenizer.events[range.1].point, link: None }],
    );

    if range.2 != 0 {
        tokenizer.map.add(
            range.2,
            0,
            vec![Event { kind: Kind::Enter, name: value_name, point: tokenizer.events[range.2].point, link: None }],
        );
        debug_assert_ne!(range.3, 0);

        if !in_delimiter_row {
            tokenizer.events[range.2].link = Some(Link::new(Content::Text));

            if range.3 > range.2 + 1 {
                let a = range.2 + 1;
                let b = range.3 - range.2 - 1;
                tokenizer.map.add(a, b, vec![]);
            }
        }

        tokenizer.map.add(
            range.3 + 1,
            0,
            vec![Event { kind: Kind::Exit, name: value_name, point: tokenizer.events[range.3].point, link: None }],
        );
    }

    if let Some(row_end) = row_end {
        tokenizer.map.add(
            row_end,
            0,
            vec![Event { kind: Kind::Exit, name: group_name, point: tokenizer.events[row_end].point, link: None }],
        );
    }
}

fn flush_table_end(tokenizer: &mut Tokenizer, index: usize, body: bool) {
    let mut exits = vec![];

    if body {
        exits.push(Event {
            kind: Kind::Exit,
            name: Name::GfmTableBody,
            point: tokenizer.events[index].point,
            link: None,
        });
    }

    exits.push(Event { kind: Kind::Exit, name: Name::GfmTable, point: tokenizer.events[index].point, link: None });

    tokenizer.map.add(index + 1, 0, exits);
}
