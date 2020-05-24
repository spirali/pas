use crate::words::{number_of_words, shortest_words, longest_words, Bound, number_of_words_zero_length, number_of_words_next_length};
use crate::dfa::Dfa;
use reduce::Reduce;
use crate::nfa::{Nfa, Transition};
use crate::common::StateId;
use crate::table::TransitionTable;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Element {
    values: Vec<usize>,
}

impl Element {

    pub fn new(n_tracks: usize) -> Self {
        Element { values: vec![0; n_tracks] }
    }

    pub fn from(values: Vec<usize>) -> Self {
        Element { values }
    }

    pub fn alphabet_size(&self) -> usize {
        1 << self.values.len()
    }

    pub fn n_tracks(self) -> usize {
        self.values.len()
    }

    pub fn push_symbol(&mut self, symbol: usize) {
        for (i, t) in self.values.iter_mut().enumerate() {
            *t <<= 1;
            *t |= (symbol >> i) & 1;
        }
    }

    pub fn pop_symbol(&mut self) {
        for t in self.values.iter_mut() {
            *t >>= 1;
        }
    }
}



fn push_tracks(tracks: &mut Vec<usize>, symbol: usize) {
    for (i, t) in tracks.iter_mut().enumerate() {
        *t <<= 1;
        *t |= (symbol >> i) & 1;
    }
}

fn pop_tracks(tracks: &mut Vec<usize>) {
    for t in tracks.iter_mut() {
        *t >>= 1;
    }
}

pub fn number_of_elements(dfa: &Dfa) -> Option<usize>
{
    let dfa = dfa.reverse().to_dfa();
    let number_of_words = number_of_words(&dfa);
    let transitions = dfa.get_state(0);
    let count = transitions[1..].iter().filter_map(|s| number_of_words[*s as usize]).reduce(|a,b| a + b);
    count.map(|v| v + if dfa.is_accepting(0) { 1 } else { 0 })
}

pub fn iterate_elements<F: FnMut(&[usize])>(dfa: &Dfa, mut limit: Option<usize>, mut callback: F) {
    if let Some(0) = limit {
        return;
    }

    let dfa = dfa.reverse().to_dfa();

    let n_tracks = dfa.n_tracks();
    if n_tracks == 0 {
        todo!()
    }

    let short = shortest_words(&dfa);
    let long = longest_words(&dfa);

    //let mut stack = Vec::new();
    let mut tracks = Vec::new();

    tracks.resize(n_tracks, 0);

    //dfa.clone().to_nfa().write_dot(std::path::Path::new("/tmp/xx.dot"), false).unwrap();

    struct ComputationDef {
        dfa: Dfa,
        short: Vec<Option<usize>>,
        long: Vec<Bound>,
    };

    let c_def = ComputationDef {
        dfa,
        short,
        long,
    };

    struct ComputationState<F: FnMut(&[usize])> {
        tracks: Vec<usize>,
        limit: Option<usize>,
        callback: F,
    };

    let mut c_state = ComputationState {
        callback,
        tracks,
        limit,
    };

    if c_def.dfa.is_accepting(0) {
        (c_state.callback)(c_state.tracks.as_slice());
        c_state.limit.as_mut().map(|v| *v -= 1);
        if let Some(0) = c_state.limit {
            return;
        }
    }

    fn compute<F: FnMut(&[usize])>(c_def: &ComputationDef, c_state: &mut ComputationState<F>, state: StateId, length: usize) -> bool {
        if length == 0 {
            if c_def.dfa.is_accepting(state) {
                (c_state.callback)(c_state.tracks.as_slice());
                c_state.limit.as_mut().map(|v| *v -= 1);
                if let Some(0) = c_state.limit {
                    return true;
                }
            }
            return false;
        }
        let new_length = length - 1;
        let transitions = c_def.dfa.get_state(state);
        for a in 0..c_def.dfa.alphabet_size() {
            let new_state = transitions[a];
            match c_def.short[new_state as usize] {
                None => continue,
                Some(x) if new_length < x => continue,
                _ => { /* Do nothing */ }
            };
            match c_def.long[new_state as usize] {
                Bound::Finite(x) if new_length > x => continue,
                _ => { /* Do nothing */ }
            };
            push_tracks(&mut c_state.tracks, a);
            if compute(c_def, c_state, new_state, new_length) {
                return true;
            }
            pop_tracks(&mut c_state.tracks);
        }
        return false;
    }

    let asize = c_def.dfa.alphabet_size();
    let mut length = 1;
    loop {
        let mut finished = true;

        for a in 1..asize /* 1 is correct here! */ {
            for t in c_state.tracks.iter_mut() {
                *t = 0;
            }
            push_tracks(&mut c_state.tracks, a);
            let new_state = c_def.dfa.get_state(0)[a];
            if Bound::Finite(length - 1) <= c_def.long[new_state as usize] {
                finished = false;
                if compute(&c_def, &mut c_state, new_state, length - 1) {
                    return;
                }
            }
        }
        if finished {
            return;
        }
        length += 1;
    }
}

pub fn get_max_value(nfa: &Nfa, track_id: usize) -> Bound {
    let mut nfa = nfa.clone();
    nfa.merge_other_tracks(track_id);
    let mut dfa = nfa.make_dfa();
    dfa.zero_suffix_closure();
    let dfa = dfa.reverse().make_dfa();
    let mut lengths = longest_words(&dfa);

    let mut state = dfa.get_state(0)[1];

    //dfa.clone().to_nfa().write_dot(std::path::Path::new("/tmp/xx.dot"), false).unwrap();

    let max = if let Bound::None = lengths[state as usize] {
        Bound::None
    } else {
        let mut value : usize = 1;
        loop {
            let s = dfa.get_state(state);
            match (lengths[s[0] as usize], lengths[s[1] as usize]) {
                (Bound::None, Bound::None) => break Bound::Finite(value),
                (Bound::Infinite, _) | (_, Bound::Infinite) => break Bound::Infinite,
                (x, y) if y < x => {
                    value <<= 1;
                    state = s[0];
                }
                (_, _) => {
                    value <<= 1;
                    value += 1;
                    state = s[1];
                }
            }
        }
    };

    if dfa.is_accepting(0) {
        max.max(Bound::Finite(0))
    } else {
        max
    }
}

pub fn cut(a_size: usize, values: Vec<usize>) -> Nfa {
    let mut data = vec![];

    let length = values.iter().map(|x| std::mem::size_of_val(x) - x.leading_zeros() as usize).max().unwrap();
    for i in (0..length).rev() {
        let mut sym = 0;
        for v in values {
            sym += (*v >> i) & 1;
        }
        for _ in 0..sym + 1 {
            data.push(Transition::simple((i + 1) as StateId));
        }
        for _ in sym + 1..a_size {
            data.push(Transition::simple((length + 1) as StateId));
        }
    }

    for _ in 0..a_size {
        data.push(Transition::simple((length + 1) as StateId)); /* Self loop */
    }

    let mut accepting = vec![true; length + 2];
    accepting[length + 1] = false;

    let nfa = Nfa::new(TransitionTable::new(values.len(), data), accepting, Nfa::simple_init());
    nfa.to_dfa().reverse()
}

pub fn get_nth_element(dfa: &Dfa, mut nth_element: usize) -> Vec<usize> {
    let mut tracks = vec![0; dfa.n_tracks()];
    if dfa.is_accepting(0) {
        if nth_element == 0 {
            /* Handle special case */
            return tracks;
        }
        nth_element -= 1;
    }

    let dfa = dfa.reverse().to_dfa();

    let mut target_len = 0;
    let mut lengths = vec![number_of_words_zero_length(&dfa)];

    let transitions = dfa.get_state(0);
    let mut remaining = nth_element;
    let mut sym = 0;

    loop {
        let lens = &lengths[lengths.len() - 1];
        for (i, tr) in transitions[1..].iter().enumerate() {
            let len = lens[*tr as usize];
            if remaining < len {
                sym = i + 1;
                break;
            }
            remaining -= len;
        }
        if sym != 0 {
            break;
        }
        assert!(lengths.len() < 64);
        /*let count : usize = transitions[1..].iter().map(|s| lengths[target_len][*s as usize]).sum();
        if remaining < count {
            break;
        } else {
            remaining -= count;
        }*/
        lengths.push(number_of_words_next_length(&dfa, &lengths[target_len]));
        target_len += 1;
    }

    push_tracks(&mut tracks, sym);
    let mut state = dfa.get_state(0)[sym];
    lengths.pop();

    for lens in lengths.iter().rev() {
        let mut found = false;
        for (i, tr) in dfa.get_state(state).iter().enumerate() {
            let len = lens[*tr as usize];
            if remaining < len {
                push_tracks(&mut tracks, i);
                state = dfa.get_state(state)[i];
                found = true;
                break;
            }
            remaining -= len;
        }
        debug_assert!(found);
    }
    tracks
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse_formula, parse_setdef};
    use crate::name::Name;
    use crate::solver::build_set;
    use crate::nfa::Transition;
    use crate::table::TransitionTable;

    fn collect_elements(dfa: &Dfa, limit: Option<usize>) -> Vec<Vec<usize>> {
        let mut result = Vec::new();
        iterate_elements(dfa, limit, |w| result.push(w.to_vec()));
        result
    }


    #[test]
    fn test_words_list1() {
        //let a = build_set(&parse_setdef("{ x | x == 220 or x == 10}")).to_dfa();

        /*let a = build_set(&parse_setdef("{ x | x == 220 or x == 10}")).to_dfa();
        assert!(collect_words(&a, Some(0)).is_empty());
        assert_eq!(collect_words(&a, Some(7)), vec![vec![220], vec![10]]);
        assert_eq!(collect_words(&a, Some(1)), vec![vec![220]]);*/
        let a = build_set(&parse_setdef("{ x, y | x > 3 and (x + y == 10)}")).to_dfa();
        println!("{}", collect_elements(&a, Some(700)).len());
        assert_eq!(collect_elements(&a, Some(3)), vec![vec![7, 3], vec![5, 5], vec![6, 4]]);
    }

    #[test]
    fn test_words_list2() {
        //let a = build_set(&parse_setdef("{ x | x == 220 or x == 10}")).to_dfa();

        /*let a = build_set(&parse_setdef("{ x | x == 220 or x == 10}")).to_dfa();
        assert!(collect_words(&a, Some(0)).is_empty());
        assert_eq!(collect_words(&a, Some(7)), vec![vec![220], vec![10]]);
        assert_eq!(collect_words(&a, Some(1)), vec![vec![220]]);*/

        let a = build_set(&parse_setdef("{ x, y | 11 * x == 3 * y and not (x == 0) }")).to_dfa();
        println!("{}", collect_elements(&a, Some(2)).len());
        assert_eq!(collect_elements(&a, Some(3)), vec![vec![3, 11], vec![6, 22], vec![9, 33]]);
    }

    #[test]
    fn test_words_list3() {
        let a = build_set(&parse_setdef("{ x | x == 1 }")).to_dfa();
        assert_eq!(collect_elements(&a, Some(2)), vec![vec![1]]);

        let a = build_set(&parse_setdef("{ x | x == 2 }")).to_dfa();
        assert_eq!(collect_elements(&a, Some(2)), vec![vec![2]]);

        let a = build_set(&parse_setdef("{ x | x == 1234567 }")).to_dfa();
        assert_eq!(collect_elements(&a, Some(2)), vec![vec![1234567]]);

        let a = build_set(&parse_setdef("{ x | x == 314 or x == 25 }")).to_dfa();
        assert_eq!(collect_elements(&a, Some(2)), vec![vec![25], vec![314]]);
    }

    #[test]
    fn test_range() {
        let a = build_set(&parse_setdef("{ x, y | x == 10}")).to_nfa();
        assert_eq!(get_max_value(&a, 0), Bound::Finite(10));
        assert_eq!(get_max_value(&a, 1), Bound::Infinite);

        let a = build_set(&parse_setdef("{ x, y | x < 0}")).to_nfa();
        assert_eq!(get_max_value(&a, 0), Bound::None);

        let a = build_set(&parse_setdef("{ x, y | x < 1}")).to_nfa();
        assert_eq!(get_max_value(&a, 0), Bound::Finite(0));

        let a = build_set(&parse_setdef("{ x, y | x < 2}")).to_nfa();
        assert_eq!(get_max_value(&a, 0), Bound::Finite(1));

        let a = build_set(&parse_setdef("{ x, y | x < 12 or x == 123}")).to_nfa();
        assert_eq!(get_max_value(&a, 0), Bound::Finite(123));

        let a = build_set(&parse_setdef("{ x | x == a + b and a < 10 and b < a + 4 and u + v == x and u == v}")).to_nfa();
        assert_eq!(get_max_value(&a, 0), Bound::Finite(20));

        let a = build_set(&parse_setdef("{ x | x == 72300 or x == 23 or x > 512}")).to_nfa();
        assert_eq!(get_max_value(&a, 0), Bound::Infinite);
    }

    #[test]
    fn test_size() {
        let a = build_set(&parse_setdef("{ x | x == 1}")).to_nfa();
        assert_eq!(number_of_elements(&a.to_dfa()), Some(1));
        let a = build_set(&parse_setdef("{ x | x == 0}")).to_nfa();
        assert_eq!(number_of_elements(&a.to_dfa()), Some(1));
        let a = build_set(&parse_setdef("{ x | not (x == x)}")).to_nfa();
        assert_eq!(number_of_elements(&a.to_dfa()), Some(0));
        let a = build_set(&parse_setdef("{ x | x < 10}")).to_nfa();
        assert_eq!(number_of_elements(&a.to_dfa()), Some(10));
        let a = build_set(&parse_setdef("{ x | x < 10 and not x == 1}")).to_nfa();
        assert_eq!(number_of_elements(&a.to_dfa()), Some(9));
        let a = build_set(&parse_setdef("{ x, y | x < 100 and y < 100}")).to_nfa();
        assert_eq!(number_of_elements(&a.to_dfa()), Some(10000));
        let a = build_set(&parse_setdef("{ x, y | x < 100 and y < 100 and not (x == y) or (x == 123 and y == 321)}")).to_nfa();
        assert_eq!(number_of_elements(&a.to_dfa()), Some(9901));
    }

    #[test]
    fn test_nth_element() {
        let a = build_set(&parse_setdef("{ x | x == 0}")).to_dfa();
        assert_eq!(get_nth_element(&a, 0), vec![0]);

        let a = build_set(&parse_setdef("{ x | x == 1}")).to_dfa();
        assert_eq!(get_nth_element(&a, 0), vec![1]);

        let a = build_set(&parse_setdef("{ x | x == 1 or x == 3}")).to_dfa();
        assert_eq!(get_nth_element(&a, 0), vec![1]);
        assert_eq!(get_nth_element(&a, 1), vec![3]);

        let a = build_set(&parse_setdef("{ x | x < 10}")).to_dfa();
        assert_eq!(get_nth_element(&a, 0), vec![0]);
        assert_eq!(get_nth_element(&a, 1), vec![1]);
        assert_eq!(get_nth_element(&a, 2), vec![2]);
        assert_eq!(get_nth_element(&a, 9), vec![9]);

        let a = build_set(&parse_setdef("{ x | x > 55}")).to_dfa();
        assert_eq!(get_nth_element(&a, 0), vec![56]);
        assert_eq!(get_nth_element(&a, 1), vec![57]);
        assert_eq!(get_nth_element(&a, 2), vec![58]);
        assert_eq!(get_nth_element(&a, 12077), vec![56 + 12077]);

        let a = build_set(&parse_setdef("{ x | 111 * y == x and x > 100 and (not exists(z)(2 * z == x))}")).to_dfa();

        assert_eq!(get_nth_element(&a, 0), vec![111]);
        assert_eq!(get_nth_element(&a, 1), vec![333]);
        assert_eq!(get_nth_element(&a, 2), vec![555]);

        let a = build_set(&parse_setdef("{ x, y | x == y + 1}")).to_dfa();
        assert_eq!(get_nth_element(&a, 0), vec![1, 0]);
        assert_eq!(get_nth_element(&a, 1), vec![2, 1]);
        assert_eq!(get_nth_element(&a, 2), vec![3, 2]);
        assert_eq!(get_nth_element(&a, 7001), vec![7002, 7001]);
    }
}