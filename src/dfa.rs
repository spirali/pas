

use hashbrown;
use crate::common::{StateId};
use hashbrown::{HashSet, HashMap};
use crate::nfa::{Transition, Nfa};
use crate::table::{TransitionTable};

#[derive(Debug, Clone)]
pub struct Dfa {
    table: TransitionTable<StateId>,
    accepting: Vec<bool>,
}

impl Dfa {
    pub fn new(table: TransitionTable<StateId>, accepting: Vec<bool>) -> Self {
        assert_eq!(table.n_states(), accepting.len());
        Dfa {
            table,
            accepting
        }
    }

    pub fn trivial(accepting: bool) -> Self {
        let table = TransitionTable::new(0, vec![0]);
        Self::new(table, vec![accepting])
    }

    pub fn neg(mut self) -> Self {
        for a in self.accepting.iter_mut() {
            *a = !*a;
        }
        Dfa {
            table: self.table,
            accepting: self.accepting,
        }
    }

    #[inline]
    pub fn n_tracks(&self) -> usize {
        self.table.n_tracks()
    }

    #[inline]
    pub fn alphabet_size(&self) -> usize {
        self.table.alphabet_size()
    }

    #[inline]
    pub fn n_states(&self) -> usize {
        self.accepting.len()
    }

    #[inline]
    pub fn get_state(&self, state_id: StateId) -> &[StateId] {
        self.table.get_state(state_id)
    }

    #[inline]
    pub fn states_and_acc(&self) -> impl Iterator<Item=(&[StateId], bool)> {
        self.table.states().zip(self.accepting.iter().copied())
    }

    #[inline]
    pub fn states(&self) -> impl Iterator<Item=&[StateId]> {
        self.table.states()
    }

    #[inline]
    pub fn is_accepting(&self, state: StateId) -> bool {
        self.accepting[state as usize]
    }

    #[inline]
    pub fn accepting(&self) -> &Vec<bool> {
        &self.accepting
    }

    #[inline]
    pub fn transitions(&self) -> &[StateId] {
        self.table.as_slice()
    }

    pub fn add_track(&mut self) {
        self.table = self.table.add_track();
    }

    pub fn swap_tracks(&mut self, index1: usize, index2: usize) {
        self.table.swap_tracks(index1, index2);
    }

    pub fn to_nfa(self) -> Nfa {
        let mut initial_states = HashSet::default();
        initial_states.insert(0);
        Nfa::new(self.table.map_states(Transition::simple), self.accepting, initial_states)
    }

    pub fn reverse_table(&self) -> TransitionTable<Transition>
    {
        let mut states = Vec::new();
        states.resize(self.table.size(), Transition::empty());
        let mut table = TransitionTable::new(self.table.n_tracks(), states);

        for (idx, state) in self.table.states().enumerate() {
            for (idx2, t) in state.iter().enumerate() {
                table.get_state_mut(*t)[idx2].states.push(idx as StateId);
            }
        }

        table
    }

    pub fn reverse_reachability(&self) -> Vec<Vec<StateId>>
    {
        let mut output = Vec::new();
        output.resize(self.table.size(), Vec::new());
        for (idx, state) in self.table.states().enumerate() {
            for t in state.iter() {
                output[*t as usize].push(idx as StateId);
            }
        }
        output
    }

    pub fn reverse(&self) -> Nfa {
        let table = self.reverse_table();

        let mut accepting = Vec::new();
        accepting.resize(self.accepting.len(), false);
        accepting[0] = true;

        let initial_states : HashSet<_> = self.accepting.iter().enumerate().filter_map(|(idx, a)| if *a { Some(idx as StateId) } else { None }).collect();
        Nfa::new(table, accepting, initial_states)
    }

    pub fn test_input<I: Iterator<Item=usize>>(&self, word: I) -> bool {
        let mut state : StateId = 0;
        for a in word {
            state = self.table.get_state(state)[a];
        }

        /*
        /* Add infinite zero suffix */
        loop {
            let mut prev = state;
            state = self.table.get_state(state)[0];
            if prev == state {
                break;
            }
        }*/
        self.accepting[state as usize]

    }

    pub fn zero_suffix_closure(&mut self) {
        let mut repeat = true;
        while repeat {
            repeat = false;
            for (i, states) in self.table.states().enumerate() {
                if self.accepting[i] {
                   continue;
                }
                let s = states[0];
                if self.accepting[s as usize] {
                    self.accepting[i] = true;
                    repeat = true;
                }
            }
        }
    }

    pub fn minimize(&self) -> Self {
        let n_states = self.accepting.len();
        assert!(n_states > 0);
        let asize = self.alphabet_size();
        let mut partitions : Vec<StateId> = self.accepting.iter().map(|a| if *a { 0 } else { 1 }).collect();

        let mut target_ids : Vec<StateId> = vec![0; asize * n_states];


        let mut prev_ids = 0;
        loop {
            self.table.fill_partitions(&partitions, &mut target_ids);
            let mut map = HashMap::new();
            let mut new_id = 0;
            for (s, acc) in self.accepting.iter().enumerate() {
                let slice = &target_ids[s * asize..(s + 1) * asize];
                let id = *map.entry((slice, acc)).or_insert_with(|| {
                    let id = new_id;
                    new_id += 1;
                    id
                });
                partitions[s as usize] = id;
            }
            if prev_ids == new_id {
                break;
            } else {
                prev_ids = new_id;
            }
        }

        let mut transitions = vec![0; asize * prev_ids as usize];
        let mut accepting = vec![false; prev_ids as usize];

        for s in 0..n_states {
            let p = partitions[s] as usize;
            let slice = &target_ids[s * asize..(s + 1) * asize];
            transitions[p * asize..(p + 1) * asize].copy_from_slice(slice);
            accepting[p] = self.accepting[s];
        }
        Dfa::new(TransitionTable::new(self.n_tracks(), transitions), accepting)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn make_da(n_tracks: usize, tr: Vec<StateId>, acc: Vec<bool>) -> Dfa {
        Dfa::new(TransitionTable::new(n_tracks, tr), acc)
    }

    #[test]
    fn test_minimize_simple() {
        let tr = vec![0, 1, 1, 0];
        let acc = vec![false, true];
        let a = make_da(1, tr, acc);

        let m = a.minimize();
        assert_eq!(m.n_states(), 2);
        assert_eq!(*m.accepting(), vec![false, true]);
    }

    #[test]
    fn test_minimize_acc() {
        //assert_eq!(*m.transitions(), vec![0, 1, 1, 0]);

        let tr = vec![0, 1, 1, 0];
        let acc = vec![true, true];
        let a = make_da(1, tr, acc);
        let m = a.minimize();
        assert_eq!(m.n_states(), 1);
        assert_eq!(m.accepting, vec![true]);
        assert_eq!(m.transitions(), vec![0, 0].as_slice());
    }

    #[test]
    fn test_minimize_nonacc() {
        let tr = vec![0, 1, 0, 1, 1, 0, 0, 0];
        let acc = vec![false, false];
        let a = make_da(2, tr, acc);
        let m = a.minimize();
        assert_eq!(m.n_states(), 1);
        assert_eq!(m.accepting, vec![false]);
        assert_eq!(m.transitions(), vec![0, 0, 0, 0].as_slice());
    }

    #[test]
    fn test_minimize_automaton1() {
        /*

           s0----S1 ===> S2  LOOP
           |
           v
           s3<->s4
           |     |
            s5   s6
            \    /
             \  /
              s7
         */

        let tr = vec![1, 3, // 0
                      2, 2, // 1
                      2, 2, // 2
                      5, 4, // 3
                      6, 3, // 4
                      5, 7, // 5
                      6, 7, // 6
                      7, 0, // 7
                    ];
        let mut acc = vec![false; 8];
        acc[1] = true;
        acc[2] = true;

        let a = make_da(1, tr, acc);
        let m = a.minimize();
        assert_eq!(m.n_states(), 5);
        assert_eq!(m.transitions(), vec![1, 2, 1, 1, 3, 2, 3, 4, 4, 0].as_slice());
    }
}


