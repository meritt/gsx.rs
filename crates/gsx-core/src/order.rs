pub(crate) fn sort_by_key<T>(items: &mut [T], key: impl Fn(&T) -> u64) {
    if items.is_sorted_by_key(&key) {
        return;
    }
    let keys = items.iter().enumerate().map(|(index, item)| (key(item), index as u32)).collect();
    swaps(keys, &mut |at, from| items.swap(at, from));
}

pub(crate) const fn pair(high: usize, low: usize) -> u64 {
    (high as u64) << 32 | low as u64
}

fn swaps(mut keys: Vec<(u64, u32)>, swap: &mut dyn FnMut(usize, usize)) {
    const DONE: u32 = u32::MAX;
    keys.sort_unstable();
    let mut order: Vec<u32> = keys.into_iter().map(|(_, index)| index).collect();
    for start in 0..order.len() {
        let mut at = start;
        loop {
            let from = order[at];
            order[at] = DONE;
            if from == DONE || from as usize == start {
                break;
            }
            swap(at, from as usize);
            at = from as usize;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::sort_by_key;

    #[test]
    fn sorts_stably_by_key() {
        let mut items: Vec<(u64, char)> =
            vec![(3, 'a'), (1, 'b'), (3, 'c'), (0, 'd'), (1, 'e'), (2, 'f'), (0, 'g'), (3, 'h')];
        let mut expected = items.clone();
        expected.sort_by_key(|item| item.0);
        sort_by_key(&mut items, |item| item.0);
        assert_eq!(items, expected);
        let mut empty: Vec<u64> = vec![];
        sort_by_key(&mut empty, |&item| item);
        assert_eq!(empty, Vec::<u64>::new());
    }
}
