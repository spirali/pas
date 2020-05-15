use crate::dfa::Dfa;
use crate::nfa::Nfa;
use crate::common::{StateId};
use hashbrown::{HashMap, HashSet};

pub fn iterate_words<F: FnMut(&[usize])>(dfa: &Dfa, mut limit: Option<usize>, mut callback: F) {

    if let Some(0) = limit {
        return;
    }

    let mut stack = Vec::new();
    let mut tracks = Vec::new();

    let n_tracks = dfa.n_tracks();
    if n_tracks == 0 {
        todo!()
    }
    tracks.resize(n_tracks, 0);
    /*if dfa.is_accepting(0) {
        callback(tracks.as_slice());
        limit.as_mut().map(|v| *v -= 1);
        if let Some(0) = limit {
            return;
        }
    }*/

    stack.push((0, 0));
    let a_size = dfa.alphabet_size();

    while let Some((state, mut sym)) = stack.pop() {
        //dbg!(&stack);
        let ssize = stack.len();
        let mask = 1 << ssize;

        if sym == 0 && dfa.get_state(state)[sym] == state {
            if dfa.is_accepting(state) /*&& tracks.iter().any(|x| (*x & mask) > 0)*/ {
                callback(tracks.as_slice());
                limit.as_mut().map(|v| *v -= 1);
                if let Some(0) = limit {
                    return;
                }
            }
            sym = a_size
        }

        if sym == a_size {
            for t in tracks.iter_mut() {
                *t = *t & !mask;
            }
            continue;
        }

        for (i, t) in tracks.iter_mut().enumerate() {
            *t = (*t & !mask) | (((sym & (1 << i)) >> i) << ssize);
        }

        let new_state = dfa.get_state(state)[sym];
        stack.push((state, sym + 1));
        stack.push((new_state, 0));
    }
}


/*pub fn number_of_words(dfa: &Dfa) -> Vec<Value>
{
    let mut output = vec![Value::Infinite; dfa.n_states()];
    let mut remaining = vec![dfa.alphabet_size(); dfa.n_states()];
    let r_table = dfa.reverse_table();
    let mut s_next = Vec::<StateId>::with_capacity(dfa.n_states());

    dbg!(&dfa);
    dbg!(&r_table);

    let mut process = |state_id: StateId, s_next: &mut Vec<StateId>, remaining: &mut Vec<usize>| {
        for s2 in r_table.get_state(state_id) {
            for s3 in &s2.states {
                if remaining[*s3 as usize] <= 1 {
                    assert_eq!(remaining[*s3 as usize], 1);
                    s_next.push(*s3);
                } else {
                    remaining[*s3 as usize] -= 1;
                }
            }
        }
    };

    for (i, (s, a)) in dfa.states().enumerate() {
        let state = i as StateId;
        if !a && s.iter().all(|x| *x == state) {
            output[i] = Value::Finite(0);
            remaining[i] += 1; // To prevent it rerunning process again
            process(state, &mut s_next, &mut remaining);
        }
    }

    //dbg!(s_next);

    let mut s_current = Vec::<StateId>::with_capacity(dfa.n_states());
    while !s_next.is_empty() {
        std::mem::swap(&mut s_current, &mut s_next);
        s_next.clear();
        for s in &s_current {
            let mut count : usize = dfa.get_state(*s).iter().map(|c| output[*c as usize].as_usize().unwrap()).sum();
            if dfa.is_accepting(*s) {
                count += 1;
            }
            output[*s as usize] = Value::Finite(count);
            process(*s, &mut s_next, &mut remaining);
        }
    }

    output
}*/

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Copy)]
pub enum Bound {
    None,
    Finite(usize),
    Infinite,
}

impl ToString for Bound {
    fn to_string(&self) -> String {
        match self {
            Self::None => "None".to_string(),
            Self::Infinite => "Infinite".to_string(),
            Self::Finite(x) => x.to_string(),
        }
    }
}

pub fn longest_words(dfa: &Dfa) -> Vec<Bound>
{
    let mut output = vec![Bound::Infinite; dfa.n_states()];
    let mut remaining = vec![dfa.alphabet_size(); dfa.n_states()];
    let r_table = dfa.reverse_table();
    let mut s_next = Vec::<StateId>::with_capacity(dfa.n_states());

    let mut process = |state_id: StateId, s_next: &mut Vec<StateId>, remaining: &mut Vec<usize>| {
        for s2 in r_table.get_state(state_id) {
            for s3 in &s2.states {
                if remaining[*s3 as usize] <= 1 {
                    assert_eq!(remaining[*s3 as usize], 1);
                    s_next.push(*s3);
                } else {
                    remaining[*s3 as usize] -= 1;
                }
            }
        }
    };

    for (i, (s, a)) in dfa.states().enumerate() {
        let state = i as StateId;
        if !a && s.iter().all(|x| *x == state) {
            output[i] = Bound::None;
            remaining[i] += 1; // To prevent it rerunning process again
            process(state, &mut s_next, &mut remaining);
        }
    }

    //dbg!(s_next);

    let mut s_current = Vec::<StateId>::with_capacity(dfa.n_states());
    while !s_next.is_empty() {
        std::mem::swap(&mut s_current, &mut s_next);
        s_next.clear();
        for s in &s_current {
            let mut length : Bound = match dfa.get_state(*s).iter().map(|c| output[*c as usize]).max().unwrap() {
                Bound::Finite(x) => Bound::Finite(x + 1),
                bound => bound,
            };
            if dfa.is_accepting(*s) {
                length = length.max(Bound::Finite(0))
            }
            output[*s as usize] = length;
            process(*s, &mut s_next, &mut remaining);
        }
    }

    output
}


pub fn get_max(nfa: &Nfa, track_id: usize) -> Bound {
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse_formula, parse_setdef};
    use crate::name::Name;
    use crate::solver::build_set;
    use crate::nfa::Transition;
    use crate::table::TransitionTable;

    fn collect_words(dfa: &Dfa, limit: Option<usize>) -> Vec<Vec<usize>> {
        let mut result = Vec::new();
        iterate_words(dfa, limit, |w| result.push(w.to_vec()));
        result
    }


    #[test]
    fn test_words_list() {
        //let a = build_set(&parse_setdef("{ x | x == 220 or x == 10}")).to_dfa();

        /*let a = build_set(&parse_setdef("{ x | x == 220 or x == 10}")).to_dfa();
        assert!(collect_words(&a, Some(0)).is_empty());
        assert_eq!(collect_words(&a, Some(7)), vec![vec![220], vec![10]]);
        assert_eq!(collect_words(&a, Some(1)), vec![vec![220]]);*/

        let a = build_set(&parse_setdef("{ x, y | (x < 10 and y < 3) or (x + y == 10)}")).to_dfa();
        println!("{}", collect_words(&a, Some(700)).len());
        assert_eq!(collect_words(&a, Some(700)), vec![vec![220], vec![10]]);
        assert_eq!(collect_words(&a, Some(700)), vec![vec![220], vec![10]]);
    }

    #[test]
    fn test_longests_words() {
        let dfa = Dfa::new(TransitionTable::new(1, vec![0, 1, 1, 0]), vec![true, false]);
        assert_eq!(longest_words(&dfa), vec![Bound::Infinite, Bound::Infinite]);
        let dfa = Dfa::new(TransitionTable::new(1, vec![1, 1, 1, 1]), vec![false, false]);
        assert_eq!(longest_words(&dfa), vec![Bound::None, Bound::None]);
        let dfa = Dfa::new(TransitionTable::new(1, vec![1, 1, 1, 1]), vec![true, false]);
        assert_eq!(longest_words(&dfa), vec![Bound::Finite(0), Bound::None]);
        let dfa = Dfa::new(TransitionTable::new(1, vec![1, 1, 2, 2, 2, 2]), vec![false, true, false]);
        assert_eq!(longest_words(&dfa), vec![Bound::Finite(1), Bound::Finite(0), Bound::None]);
        let dfa = Dfa::new(TransitionTable::new(1, vec![1, 1, 2, 2, 2, 2]), vec![true, true, false]);
        assert_eq!(longest_words(&dfa), vec![Bound::Finite(1), Bound::Finite(0), Bound::None]);
    }

    #[test]
    fn test_range() {
        let a = build_set(&parse_setdef("{ x, y | x == 10}")).to_nfa();
        assert_eq!(get_max(&a, 0), Bound::Finite(10));
        assert_eq!(get_max(&a, 1), Bound::Infinite);

        let a = build_set(&parse_setdef("{ x, y | x < 0}")).to_nfa();
        assert_eq!(get_max(&a, 0), Bound::None);

        let a = build_set(&parse_setdef("{ x, y | x < 1}")).to_nfa();
        assert_eq!(get_max(&a, 0), Bound::Finite(0));

        let a = build_set(&parse_setdef("{ x, y | x < 2}")).to_nfa();
        assert_eq!(get_max(&a, 0), Bound::Finite(1));

        let a = build_set(&parse_setdef("{ x, y | x < 12 or x == 123}")).to_nfa();
        assert_eq!(get_max(&a, 0), Bound::Finite(123));

        let a = build_set(&parse_setdef("{ x | x == a + b and a < 10 and b < a + 4 and u + v == x and u == v}")).to_nfa();
        assert_eq!(get_max(&a, 0), Bound::Finite(20));

        let a = build_set(&parse_setdef("{ x | x == 72300 or x == 23 or x > 512}")).to_nfa();
        assert_eq!(get_max(&a, 0), Bound::Infinite);
    }
}