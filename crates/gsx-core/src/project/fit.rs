use super::table::Search;

pub const WINDOW: usize = 64;

pub fn search(
    mode: Search,
    upper: usize,
    start: Option<usize>,
    clean: impl Fn(usize) -> bool,
    mut probe: impl FnMut(usize) -> (bool, f64),
) -> Option<usize> {
    if mode == Search::Exhaustive {
        return (0..=upper).rev().find(|&keep| probe(keep).0);
    }
    let (guess, above) = if let Some(guess) = start {
        (guess.min(upper), upper + 1)
    } else {
        let (fit, load) = probe(upper);
        if fit {
            return Some(upper);
        }
        (((upper as f64 / load) as usize).min(upper.saturating_sub(1)), upper)
    };
    let (fit, load) = probe(guess);
    let mut fits = |keep| probe(keep).0;
    let step = ((guess as f64 / load) as usize).abs_diff(guess).clamp(1, upper.max(1));
    let (mut below, mut above, mut step) = (guess, above, step);
    if fit {
        while below + step < above && fits(below + step) {
            below += step;
            step *= 2;
        }
        above = above.min(below + step);
    } else {
        loop {
            if below == 0 {
                return None;
            }
            above = below;
            below = below.saturating_sub(step);
            step *= 2;
            if fits(below) {
                break;
            }
        }
    }
    while above - below > 1 {
        let middle = below + (above - below) / 2;
        if fits(middle) {
            below = middle;
        } else {
            above = middle;
        }
    }
    let end = upper.min(below + WINDOW);
    let top = (below + 1..end).find(|&keep| clean(keep)).unwrap_or(end);
    match (below + 1..=top).rev().find(|&keep| fits(keep)) {
        Some(keep) if keep == top && top < end => (top + 1..=end).rev().find(|&keep| fits(keep)).or(Some(top)),
        found => found.or(Some(below)),
    }
}
