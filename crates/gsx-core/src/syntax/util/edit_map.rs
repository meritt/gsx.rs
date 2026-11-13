use crate::syntax::event::Event;

fn shift_links(events: &mut [Event], jumps: &[(usize, usize, usize)]) {
    let mut jump_index = 0;
    let mut index = 0;
    let mut add = 0;
    let mut rm = 0;

    while index < events.len() {
        let rm_curr = rm;

        while jump_index < jumps.len() && jumps[jump_index].0 <= index {
            add = jumps[jump_index].2;
            rm = jumps[jump_index].1;
            jump_index += 1;
        }

        if rm > rm_curr {
            index += rm - rm_curr;
        } else {
            if let Some(link) = &events[index].link
                && let Some(next) = link.next()
            {
                events[next].link.as_mut().unwrap().set_previous(Some(index + add - rm));

                while jump_index < jumps.len() && jumps[jump_index].0 <= next {
                    add = jumps[jump_index].2;
                    rm = jumps[jump_index].1;
                    jump_index += 1;
                }

                events[index].link.as_mut().unwrap().set_next(Some(next + add - rm));
                index = next;
                continue;
            }

            index += 1;
        }
    }
}

#[derive(Debug)]
struct Edit {
    at: usize,
    rank: isize,
    remove: usize,
    add: Vec<Event>,
}

#[derive(Debug)]
pub struct EditMap {
    edits: Vec<Edit>,
}

impl EditMap {
    pub fn new() -> EditMap {
        EditMap { edits: vec![] }
    }

    pub fn add(&mut self, index: usize, remove: usize, add: Vec<Event>) {
        self.add_impl(index, remove, add, false);
    }

    pub fn add_before(&mut self, index: usize, remove: usize, add: Vec<Event>) {
        self.add_impl(index, remove, add, true);
    }

    pub fn consume(&mut self, events: &mut Vec<Event>) {
        if self.edits.is_empty() {
            return;
        }
        if !self.edits.is_sorted_by_key(|edit| (edit.at, edit.rank)) {
            self.edits.sort_unstable_by_key(|edit| (edit.at, edit.rank));
        }

        self.edits.dedup_by(|edit, last| {
            let same = last.at == edit.at;
            if same {
                last.remove += edit.remove;
                last.add.append(&mut edit.add);
            }
            same
        });

        let mut add_acc = 0;
        let mut remove_acc = 0;
        let jumps: Vec<_> = self
            .edits
            .iter()
            .map(|edit| {
                remove_acc += edit.remove;
                add_acc += edit.add.len();
                (edit.at, remove_acc, add_acc)
            })
            .collect();

        shift_links(events, &jumps);

        let mut out = Vec::with_capacity(events.len() + add_acc - remove_acc);
        let mut cursor = 0;
        for edit in &self.edits {
            out.extend_from_slice(&events[cursor..edit.at]);
            out.extend_from_slice(&edit.add);
            cursor = edit.at + edit.remove;
        }
        out.extend_from_slice(&events[cursor..]);
        *events = out;
        self.edits.clear();
    }

    fn add_impl(&mut self, at: usize, remove: usize, add: Vec<Event>, before: bool) {
        if remove == 0 && add.is_empty() {
            return;
        }
        let order = self.edits.len().cast_signed();
        self.edits.push(Edit { at, rank: if before { -order - 1 } else { order }, remove, add });
    }
}
