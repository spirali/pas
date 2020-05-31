use crate::automaton::Automaton;
use crate::nfa::{Transition, Nfa};
use crate::table::TransitionTable;
use crate::dfa::Dfa;
use crate::name::Name;
use hashbrown::HashMap;
use crate::common::StateId;
use crate::elements::{Element, cut, get_nth_element, number_of_elements};

#[derive(Debug, Clone)]
pub struct AutomaticSet {
    automaton: Automaton,
    track_names: Vec<Name>,
}

impl AutomaticSet {

    pub fn singleton(track_name: Name, mut value: u64) -> AutomaticSet {
        let mut transitions = Vec::new();
        let mut state_id = 0;

        while value > 0 {
            state_id += 1;
            if value & 1 == 0 {
                transitions.push(Transition::simple(state_id));
                transitions.push(Transition::empty());
            } else {
                transitions.push(Transition::empty());
                transitions.push(Transition::simple(state_id));
            }
            value >>= 1;
        }

        transitions.push(Transition::simple(state_id));
        transitions.push(Transition::empty());

        let mut accepting = Vec::new();
        accepting.resize(state_id as usize + 1, false);
        accepting[state_id as usize] = true;

        let nfa = Nfa::new(TransitionTable::new(1, transitions), accepting, Nfa::simple_init());

        AutomaticSet {
            automaton: Automaton::Nfa(nfa),
            track_names: vec![track_name],
        }
    }

    pub fn double(name1: Name, name2: Name) -> AutomaticSet {
        /* name1 * 2 = name2 */
        assert_ne!(name1, name2);
        let table = TransitionTable::new(2, vec![
            0, 1, 2, 2,
            2, 2, 0, 1,
            2, 2, 2, 2,
        ]);
        AutomaticSet {
            automaton: Automaton::Dfa(Dfa::new(table, vec![true, false, false])),
            track_names: vec![name1, name2]
        }
    }

    pub fn trivial(accepting: bool) -> AutomaticSet {
        AutomaticSet {
            automaton: Automaton::Dfa(Dfa::trivial(accepting)),
            track_names: Vec::new(),
        }
    }

    pub fn equivalence(name1: Name, name2: Name) -> AutomaticSet {
        assert_ne!(name1, name2);
        let table = TransitionTable::new(2, vec![
            0, 1, 1, 0,
            1, 1, 1, 1
        ]);
        AutomaticSet {
            automaton: Automaton::Dfa(Dfa::new(table, vec![true, false])),
            track_names: vec![name1, name2]
        }
    }

    pub fn addition(name1: Name, name2: Name, name3: Name) -> AutomaticSet {
        assert_ne!(name1, name2);
        assert_ne!(name1, name3);
        assert_ne!(name2, name3);
        let t = Transition::simple;
        let e = Transition::empty;

        let table = TransitionTable::new(3, vec![
            /* 000,  001,  010,  011,  100,  101,  110,  111, */
               t(0), e(),  e(),  t(1),  e(),  t(0), t(0), e(),
               e(),  t(1), t(1), e(),   t(0), e(),  e(),  t(1),
        ]);

        AutomaticSet {
            automaton: Automaton::Nfa(Nfa::new(table, vec![true, false], Nfa::simple_init())),
            track_names: vec![name1, name2, name3]
        }
    }

    pub fn cut(&self, nth_element: usize, lte: bool) -> AutomaticSet {
        let self_dfa = self.automaton.as_dfa();
        let element = get_nth_element(&self_dfa, nth_element);
        assert_eq!(element.n_tracks(), self.track_names.len());
        let cut_nfa = cut(&element);
        let neg_cut_nfa = cut_nfa.to_dfa().neg().to_nfa();

        let mut nfa = self_dfa.neg().to_nfa();
        nfa.join(&neg_cut_nfa);
        let dfa = nfa.to_dfa().neg();

        AutomaticSet {
            track_names: self.track_names.to_vec(),
            automaton: Automaton::Dfa(dfa),
        }
    }

    pub fn cut2(&self, nth_element: usize) -> (AutomaticSet, AutomaticSet) {
        let self_dfa = self.automaton.as_dfa();
        let element = get_nth_element(&self_dfa, nth_element);
        assert_eq!(element.n_tracks(), self.track_names.len());

        let cut_nfa = cut(&element);
        let neg_cut_nfa = cut_nfa.to_dfa().neg().to_nfa();

        let mut nfa1 = self_dfa.neg().to_nfa();
        let mut nfa2 = nfa1.clone();
        nfa1.join(&neg_cut_nfa);
        let dfa1 = nfa1.to_dfa().neg();

        nfa2.join(&cut_nfa);
        let dfa2 = nfa2.to_dfa().neg();

        (AutomaticSet {
            track_names: self.track_names.to_vec(),
            automaton: Automaton::Dfa(dfa1),
        },
        AutomaticSet {
            track_names: self.track_names.to_vec(),
            automaton: Automaton::Dfa(dfa2),
        })
    }

    pub fn neg(self) -> AutomaticSet {
        AutomaticSet {
            track_names: self.track_names,
            automaton: Automaton::Dfa(self.automaton.to_dfa().neg()),
        }
    }

    pub fn track_names(&self) -> &[Name] {
        &self.track_names
    }

    pub fn union(mut self, mut other: AutomaticSet) -> AutomaticSet {
        self.synchronize_tracks(&mut other);
        let mut a1 = self.automaton.to_nfa();
        let a2 = other.automaton.to_nfa();
        a1.join(&a2);
        let r = AutomaticSet {
            track_names: self.track_names,
            automaton: Automaton::Nfa(a1),
        };

        r
    }

    pub fn intersection(self, other: AutomaticSet) -> AutomaticSet {
        self.neg().union(other.neg()).neg()
    }

    pub fn size(&self) -> Option<usize> {
        number_of_elements(&self.automaton.as_dfa())
    }

    pub fn order_tracks(&mut self, names: &[Name]) {
        for t in names {
            if !self.track_names().contains(&t) {
                self.add_track(t.clone());
            }
        }
        assert_eq!(names.len(), self.track_names.len());

        for (i, name) in names.iter().enumerate() {
            let p = self.track_names.iter().position(|t| t == name).unwrap();
            self.swap_tracks(i, p);
        }
        debug_assert_eq!(names, self.track_names.as_slice());
    }

    fn synchronize_tracks(&mut self, other: &mut AutomaticSet) {
        let track_names = self.track_names().to_vec();

        for t in other.track_names() {
            if !track_names.contains(t) {
                self.add_track(t.clone());
            }
        }
        other.order_tracks(self.track_names());

        /*
        for t in track_names {
            if !other.track_names().contains(&t) {
                other.add_track(t);
            }
        }

        assert_eq!(self.track_names.len(), other.track_names.len());

        for (i, name) in self.track_names.iter().enumerate() {
            let p = other.track_names.iter().position(|t| t == name).unwrap();
            other.swap_tracks(i, p);
        }
        debug_assert_eq!(self.track_names, other.track_names);*/
    }

    pub fn add_track(&mut self, name: Name) {
        self.track_names.push(name);
        self.automaton.add_track();
    }

    pub fn swap_tracks(&mut self, index1: usize, index2: usize) {
        assert!(index1 < self.track_names.len());
        assert!(index2 < self.track_names.len());

        if index1 == index2 {
            return;
        }
        self.track_names.swap(index1, index2);
        self.automaton.swap_tracks(index1, index2);
    }

    pub fn to_dfa(self) -> Dfa {
        self.automaton.to_dfa()
    }

    pub fn as_dfa(&self) -> Dfa {
        self.automaton.as_dfa()
    }


    pub fn to_nfa(self) -> Nfa {
        self.automaton.to_nfa()
    }

    pub fn test_input(&mut self, values: &[(&str, u64)]) -> bool {
        let map: HashMap<Name, u64> = values.into_iter().map(|(k, v)| (Name::from_str(k), *v)).collect();
        let mut values = Vec::<u64>::new();
        for name in &self.track_names {
            values.push(*map.get(&name).unwrap());
        }
        values.reverse();
        let mut tape = Vec::new();
        let mut next = true;
        while next {
            let mut v = 0;
            next = false;
            for val in values.iter_mut() {
                v <<= 1;
                v += *val & 1;
                *val >>= 1;
                next |= *val > 0;
            }
            tape.push(v as usize);
        }
        let dfa = self.automaton.ensure_dfa();
        dfa.test_input(tape.into_iter())
    }

    pub fn track_id(&self, name: Name) -> Option<usize> {
        self.track_names.iter().position(|n| n == &name)
    }

    pub fn exists(mut self, name: Name) -> AutomaticSet {
        if let Some(track) = self.track_id(name) {
            self.swap_tracks(0, track);
            let mut track_names = self.track_names;
            track_names.remove(0);
            let mut nfa = self.automaton.to_nfa();
            nfa.merge_first_track();
            nfa.zero_suffix_closure();
            AutomaticSet {
                track_names,
                automaton: Automaton::Nfa(nfa)
            }
        } else {
            self
        }
    }

    pub fn ensure_dfa(&mut self) {
        self.automaton.ensure_dfa();
    }

    pub fn is_empty(self) -> bool {
        let dfa = self.to_dfa();
        dfa.n_states() == 1 && !dfa.is_accepting(0)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::build_set;
    use crate::parser::parse_setdef;
    use crate::elements::iterate_elements;

    fn collect_elements(dfa: &Dfa, limit: Option<usize>) -> Vec<Vec<usize>> {
        let mut result = Vec::new();
        iterate_elements(dfa, limit, |w| result.push(w.as_slice().to_vec()));
        result
    }

    fn number_to_word(mut number: usize) -> Vec<usize> {
        let mut r = Vec::new();
        while number > 0 {
            r.push(number & 1);
            number >>= 1;
        }
        r
    }

    fn number_to_word2(mut number1: usize, mut number2: usize) -> Vec<usize> {
        let mut r = Vec::new();
        while number1 > 0 || number2 > 0 {
            r.push((number1 & 1) + ((number2 & 1) << 1));
            number1 >>= 1;
            number2 >>= 1;
        }
        r
    }

    #[test]
    fn test_singleton() {
        let aset = AutomaticSet::singleton(Name::from_str("x"), 0);
        let dfa = aset.to_dfa();
        assert!(dfa.test_input(Vec::<usize>::new().into_iter()));
        assert!(!dfa.test_input(vec![1].into_iter()));
        assert!(dfa.test_input(vec![0].into_iter()));

        let v = 0b010101100110;
        let aset = AutomaticSet::singleton(Name::from_str("x"), v as u64);
        let dfa = aset.to_dfa();
        assert!(!dfa.test_input(Vec::<usize>::new().into_iter()));
        assert!(dfa.test_input(number_to_word(v).into_iter()));
        assert!(!dfa.test_input(number_to_word(v + 1).into_iter()));
        assert!(!dfa.test_input(number_to_word(v - 1).into_iter()));
        assert!(!dfa.test_input(number_to_word(v * 2).into_iter()));
        assert!(!dfa.test_input(number_to_word(v / 2).into_iter()));
    }

    #[test]
    fn test_union1() {
        let aset1 = AutomaticSet::singleton(Name::from_str("x"), 1);
        let aset2 = AutomaticSet::singleton(Name::from_str("x"), 10);

        let dfa = aset1.union(aset2).to_dfa();
        assert!(dfa.test_input(number_to_word(1).into_iter()));
        assert!(!dfa.test_input(number_to_word(2).into_iter()));
        assert!(dfa.test_input(number_to_word(10).into_iter()));
    }

    #[test]
    fn test_union2() {
        let aset1 = AutomaticSet::singleton(Name::from_str("x"), 1);
        let aset2 = AutomaticSet::singleton(Name::from_str("y"), 10);
        let dfa = aset1.union(aset2).to_dfa();
        //dfa.clone().to_nfa().write_dot(std::path::Path::new("/tmp/x.dot"), false).unwrap();
        assert!(dfa.test_input(number_to_word2(1, 0).into_iter()));
        assert!(dfa.test_input(number_to_word2(1, 10).into_iter()));
        assert!(dfa.test_input(number_to_word2(0, 10).into_iter()));
        assert!(!dfa.test_input(number_to_word2(2, 0).into_iter()));
        assert!(!dfa.test_input(number_to_word2(10, 0).into_iter()));
    }

    #[test]
    fn test_double() {
        let n = Name::from_str("x");
        let m = Name::from_str("y");
        let aset1 = AutomaticSet::double(n.clone(), m.clone());
        let dfa = aset1.to_dfa();
        for i in 0..51 {
            for j in 0..71 {
                assert_eq!(dfa.test_input(number_to_word2(i, j).into_iter()), i * 2 == j);
            }
        }
    }

    #[test]
    fn test_cut() {
        let a = build_set(&parse_setdef("{ x | x == 1 or x == 3}"));
        assert_eq!(collect_elements(&a.cut(0, true).to_dfa(), None), vec![vec![1]]);
        assert_eq!(collect_elements(&a.cut(1, true).to_dfa(), None), vec![vec![1], vec![3]]);

        let a = build_set(&parse_setdef("{ x | x > 5 and x < 20 and 2 * y == x}"));
        assert_eq!(collect_elements(&a.cut(3, true).to_dfa(), None), vec![vec![6], vec![8], vec![10], vec![12]]);

        let a = build_set(&parse_setdef("{ x, y | x == y + 13 or x == y + 11}"));
        assert_eq!(collect_elements(&a.cut(5, true).to_dfa(), None), vec![vec![11, 0], vec![13, 0], vec![12, 1], vec![14, 1], vec![13, 2], vec![15, 2]]);
    }

}