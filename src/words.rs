use crate::dfa::Dfa;
use crate::nfa::Nfa;
use crate::common::{StateId};
use hashbrown::{HashMap, HashSet};
use itertools::Itertools;
use crate::words::Bound::Finite;


#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Copy)]
pub enum Bound {
    None,
    Finite(usize),
    Infinite,
}

impl Bound {

    #[inline]
    pub fn increase(&self) -> Bound {
        match self {
            Self::Finite(x) => Self::Finite(x + 1),
            x => *x
        }
    }

    pub fn to_limit(&self) -> Option<usize> {
        match self {
            Self::None => Some(0),
            Self::Finite(x) => Some(x + 1),
            Self::Infinite => None,
        }
    }
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
    let r_table = dfa.reverse_reachability();
    let mut s_next = Vec::<StateId>::with_capacity(dfa.n_states());

    let mut process = |state_id: StateId, s_next: &mut Vec<StateId>, remaining: &mut Vec<usize>| {
        for s3 in &r_table[state_id as usize] {
            if remaining[*s3 as usize] <= 1 {
                assert_eq!(remaining[*s3 as usize], 1);
                s_next.push(*s3);
            } else {
                remaining[*s3 as usize] -= 1;
            }
        }
    };

    for (i, (s, a)) in dfa.states_and_acc().enumerate() {
        let state = i as StateId;
        if !a && s.iter().all(|x| *x == state) {
            output[i] = Bound::None;
            remaining[i] += 1; // To prevent it rerunning process again
            process(state, &mut s_next, &mut remaining);
        }
    }

    let mut s_current = Vec::<StateId>::with_capacity(dfa.n_states());
    while !s_next.is_empty() {
        std::mem::swap(&mut s_current, &mut s_next);
        s_next.clear();
        for s in &s_current {
            let mut length : Bound = dfa.get_state(*s).iter().map(|c| output[*c as usize]).max().unwrap().increase();
            if dfa.is_accepting(*s) {
                length = length.max(Bound::Finite(0))
            }
            output[*s as usize] = length;
            process(*s, &mut s_next, &mut remaining);
        }
    }

    output
}

pub fn number_of_words(dfa: &Dfa) -> Vec<Option<usize>>
{
    //dfa.clone().to_nfa().write_dot(std::path::Path::new("/tmp/xx.dot"), false).unwrap();
    let mut output = vec![None; dfa.n_states()];
    let mut remaining = vec![dfa.alphabet_size(); dfa.n_states()];
    let r_table = dfa.reverse_reachability();
    let mut s_next = Vec::<StateId>::with_capacity(dfa.n_states());

    let mut process = |state_id: StateId, s_next: &mut Vec<StateId>, remaining: &mut Vec<usize>| {
        for s3 in &r_table[state_id as usize] {
            if remaining[*s3 as usize] <= 1 {
                assert_eq!(remaining[*s3 as usize], 1);
                s_next.push(*s3);
                remaining[*s3 as usize] = 0;
            } else {
                remaining[*s3 as usize] -= 1;
            }
        }
    };

    for (i, (s, a)) in dfa.states_and_acc().enumerate() {
        let state = i as StateId;
        if !a && s.iter().all(|x| *x == state) {
            output[i] = Some(0);
            remaining[i] += 1; // To prevent it rerunning process again
            process(state, &mut s_next, &mut remaining);
        }
    }

    let mut s_current = Vec::<StateId>::with_capacity(dfa.n_states());
    while !s_next.is_empty() {
        std::mem::swap(&mut s_current, &mut s_next);
        s_next.clear();
        for s in &s_current {
            let mut size : usize = dfa.get_state(*s).iter().map(|c| output[*c as usize].unwrap()).sum();
            if dfa.is_accepting(*s) {
                size += 1;
            }
            output[*s as usize] = Some(size);
            process(*s, &mut s_next, &mut remaining);
        }
    }

    output
}

pub fn shortest_words(dfa: &Dfa) -> Vec<Option<usize>>
{
    let mut output = vec![None; dfa.n_states()];
    let mut n_next = HashSet::<StateId>::with_capacity(dfa.n_states());
    let mut reverse = vec![Vec::<StateId>::new(); dfa.n_states()];

    for (i, (s, a)) in dfa.states_and_acc().enumerate() {
        let state_id = i as StateId;
        for t in s {
            reverse[*t as usize].push(state_id);
        }
        if a {
            n_next.insert(state_id);
        }
    }

    let mut step = 0;
    while !n_next.is_empty() {
        for state_id in std::mem::take(&mut n_next) {
            if output[state_id as usize].is_none() {
                output[state_id as usize] = Some(step);
                n_next.extend(reverse[state_id as usize].iter());
            }
        }
        step += 1;
    }
    output
}

pub fn number_of_words_zero_length(dfa: &Dfa) -> Vec<usize> {
    dfa.accepting().iter().map(|a| if *a { 1 } else { 0 }).collect()
}

pub fn number_of_words_next_length(dfa: &Dfa, prev: &Vec<usize>) -> Vec<usize> {
    dfa.states().map(|s| {
        s.iter().map(|t| prev[*t as usize]).sum()
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse_formula, parse_setdef};
    use crate::name::Name;
    use crate::solver::build_set;
    use crate::nfa::Transition;
    use crate::table::TransitionTable;


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
    fn test_shortest_words() {
        let dfa = Dfa::new(TransitionTable::new(1, vec![0, 1, 1, 0]), vec![true, false]);
        assert_eq!(shortest_words(&dfa), vec![Some(0), Some(1)]);

        let dfa = Dfa::new(TransitionTable::new(1, vec![0, 1, 1, 0]), vec![false, false]);
        assert_eq!(shortest_words(&dfa), vec![None, None]);

        let dfa = Dfa::new(TransitionTable::new(1, vec![0, 1, 1, 0]), vec![true, true]);
        assert_eq!(shortest_words(&dfa), vec![Some(0), Some(0)]);

        let dfa = Dfa::new(TransitionTable::new(1, vec![1, 1, 1, 1]), vec![true, false]);
        assert_eq!(shortest_words(&dfa), vec![Some(0), None]);

        let dfa = Dfa::new(TransitionTable::new(1, vec![1, 1, 1, 1]), vec![false, true]);
        assert_eq!(shortest_words(&dfa), vec![Some(1), Some(0)]);

        let dfa = Dfa::new(TransitionTable::new(1, vec![1, 2, 2, 2, 3, 3, 4, 3, 4, 4]), vec![false, false, false, false, true]);
        assert_eq!(shortest_words(&dfa), vec![Some(3), Some(3), Some(2), Some(1), Some(0)]);
    }
}