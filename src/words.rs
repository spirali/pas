use crate::dfa::Dfa;
use crate::nfa::Nfa;
use crate::common::{StateId};
use hashbrown::{HashMap, HashSet};
use itertools::Itertools;
use crate::words::Bound::Finite;
use reduce::Reduce;


pub fn iterate_words<F: FnMut(&[usize])>(dfa: &Dfa, mut limit: Option<usize>, mut callback: F) {
    if let Some(0) = limit {
        return;
    }

    let dfa = dfa.reverse().determinize().minimize();

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

    fn push(tracks: &mut Vec<usize>, symbol: usize) {
        for (i, t) in tracks.iter_mut().enumerate() {
            *t <<= 1;
            *t |= (symbol >> i) & 1;
        }
    }

    fn pop(tracks: &mut Vec<usize>) {
        for (i, t) in tracks.iter_mut().enumerate() {
            *t >>= 1;
        }
    }

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
                Finite(x) if new_length > x => continue,
                _ => { /* Do nothing */ }
            };
            push(&mut c_state.tracks, a);
            if compute(c_def, c_state, new_state, new_length) {
                return true;
            }
            pop(&mut c_state.tracks);
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
            push(&mut c_state.tracks, a);
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



    /*
    dfa.clone().to_nfa().write_dot(std::path::Path::new("/tmp/xx.dot"), false).unwrap();

    if dfa.is_accepting(0) {
        callback(tracks.as_slice());
        limit.as_mut().map(|v| *v -= 1);
        if let Some(0) = limit {
            return;
        }
    }

    dbg!(&short);
    dbg!(&long);

    let start = if let Some(x) = short[0] { x.max(1) } else {
        return;
    };


    /*let end = match long[0] {
        Bound::None => unreachable!(),
        Bound::Finite(x) => x.max(1),
        Bound::Infinite => ,
    };*/

    let a_size = dfa.alphabet_size();
    dbg!(start);
    for length in start..=std::mem::size_of::<usize>() * 8 {
        for t in tracks.iter_mut() {
            *t = 0;
        }
        stack.push((0, 1)); // Start from 1 not 0!
        let len = length;
        while let Some((state, mut step)) = stack.pop() {
            if step == a_size {
                pop(&mut tracks);
                continue;
            }
            let new_state = dfa.get_state(state)[step];
            println!("state ={} , new_state={}, {} {}", &state, &new_state, stack.len(), len);
            if stack.len() == len {
                /*dbg!(&stack);
                dbg!(&tracks);
                dbg!(&len);
                dbg!(&state);
                dbg!(&new_state);
                debug_assert!(dfa.is_accepting(new_state));*/
                /*dbg!(dfa.accepting());
                dfa.clone().to_nfa().write_dot(std::path::Path::new("/tmp/xx.dot"), false).unwrap();*/
                if dfa.is_accepting(new_state) {
                    push(&mut tracks, step);
                    callback(tracks.as_slice());
                    pop(&mut tracks);
                    limit.as_mut().map(|v| *v -= 1);
                    if let Some(0) = limit {
                        return;
                    }
                }
                continue;
            }
            stack.push((state, step + 1));
            if let Some(min) = short[new_state as usize] {
                let rest = len - stack.len();
                println!("rest={}, min={}", rest, min);
                if min < rest && Bound::Finite(rest) <= long[new_state as usize] {
                    push(&mut tracks, step);
                    stack.push((new_state, 0));
                }
            }
        }
    }*/
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
    let r_table = dfa.reverse_table();
    let mut s_next = Vec::<StateId>::with_capacity(dfa.n_states());

    let mut process = |state_id: StateId, s_next: &mut Vec<StateId>, remaining: &mut Vec<usize>| {
        for s2 in r_table.get_state(state_id) {
            for s3 in &s2.states {
                if remaining[*s3 as usize] <= 1 {
                    assert_eq!(remaining[*s3 as usize], 1);
                    s_next.push(*s3);
                    remaining[*s3 as usize] = 0;
                } else {
                    remaining[*s3 as usize] -= 1;
                }
            }
        }
    };

    for (i, (s, a)) in dfa.states().enumerate() {
        let state = i as StateId;
        if !a && s.iter().all(|x| *x == state) {
            output[i] = Some(0);
            remaining[i] += 1; // To prevent it rerunning process again
            dbg!("S", &remaining);
            process(state, &mut s_next, &mut remaining);
            dbg!("X", &remaining);
        }
    }

    let mut s_current = Vec::<StateId>::with_capacity(dfa.n_states());
    dbg!(&s_next, &remaining);
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

pub fn number_of_elements(dfa: &Dfa) -> Option<usize>
{
    let dfa = dfa.reverse().to_dfa();
    let number_of_words = number_of_words(&dfa);
    let transitions = dfa.get_state(0);
    let count = (1..dfa.alphabet_size()).into_iter().filter_map(|a| number_of_words[transitions[a] as usize]).reduce(|a,b| a + b);
    count.map(|v| v + if dfa.is_accepting(0) { 1 } else { 0 })
}

pub fn shortest_words(dfa: &Dfa) -> Vec<Option<usize>>
{
    let mut output = vec![None; dfa.n_states()];
//    let mut remaining = vec![dfa.alphabet_size(); dfa.n_states()];
    let mut n_next = HashSet::<StateId>::with_capacity(dfa.n_states());
    let mut reverse = vec![Vec::<StateId>::new(); dfa.n_states()];

    for (i, (s, a)) in dfa.states().enumerate() {
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
    fn test_words_list1() {
        //let a = build_set(&parse_setdef("{ x | x == 220 or x == 10}")).to_dfa();

        /*let a = build_set(&parse_setdef("{ x | x == 220 or x == 10}")).to_dfa();
        assert!(collect_words(&a, Some(0)).is_empty());
        assert_eq!(collect_words(&a, Some(7)), vec![vec![220], vec![10]]);
        assert_eq!(collect_words(&a, Some(1)), vec![vec![220]]);*/
        let a = build_set(&parse_setdef("{ x, y | x > 3 and (x + y == 10)}")).to_dfa();
        println!("{}", collect_words(&a, Some(700)).len());
        assert_eq!(collect_words(&a, Some(3)), vec![vec![7, 3], vec![5, 5], vec![6, 4]]);
    }

    #[test]
    fn test_words_list2() {
        //let a = build_set(&parse_setdef("{ x | x == 220 or x == 10}")).to_dfa();

        /*let a = build_set(&parse_setdef("{ x | x == 220 or x == 10}")).to_dfa();
        assert!(collect_words(&a, Some(0)).is_empty());
        assert_eq!(collect_words(&a, Some(7)), vec![vec![220], vec![10]]);
        assert_eq!(collect_words(&a, Some(1)), vec![vec![220]]);*/

        let a = build_set(&parse_setdef("{ x, y | 11 * x == 3 * y and not (x == 0) }")).to_dfa();
        println!("{}", collect_words(&a, Some(2)).len());
        assert_eq!(collect_words(&a, Some(3)), vec![vec![3, 11], vec![6, 22], vec![9, 33]]);
    }

    #[test]
    fn test_words_list3() {
        let a = build_set(&parse_setdef("{ x | x == 1 }")).to_dfa();
        assert_eq!(collect_words(&a, Some(2)), vec![vec![1]]);

        let a = build_set(&parse_setdef("{ x | x == 2 }")).to_dfa();
        assert_eq!(collect_words(&a, Some(2)), vec![vec![2]]);

        let a = build_set(&parse_setdef("{ x | x == 1234567 }")).to_dfa();
        assert_eq!(collect_words(&a, Some(2)), vec![vec![1234567]]);

        let a = build_set(&parse_setdef("{ x | x == 314 or x == 25 }")).to_dfa();
        assert_eq!(collect_words(&a, Some(2)), vec![vec![25], vec![314]]);
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
}