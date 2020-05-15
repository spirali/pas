
use crate::dfa::{Dfa};
use hashbrown::{HashSet, HashMap};
use smallvec::{smallvec, SmallVec};
use crate::common::{StateId, StateSet};
use crate::table::TransitionTable;
use std::path::Path;
use std::fs::File;
use std::io::Write;
use itertools::Itertools;

#[derive(Debug, Default, Clone)]
pub struct Transition {
    pub states: SmallVec<[StateId; 2]>
}

impl Transition {

    #[inline]
    pub fn empty() -> Transition {
        Transition {
            states: Default::default()
        }
    }

    #[inline]
    pub fn is_simple(&self, state_id: StateId) -> bool {
        self.states.len() == 1 && self.states[0] == state_id
    }

    #[inline]
    pub fn simple(state_id: StateId) -> Transition {
        Transition {
            states: smallvec![state_id]
        }
    }

    pub fn pair(state_id1: StateId, state_id2: StateId) -> Transition {
        Transition {
            states: smallvec![state_id1, state_id2]
        }
    }

    pub fn new(states: SmallVec<[StateId; 2]>) -> Transition {
        Transition {
            states,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Nfa {
    table: TransitionTable<Transition>,
    accepting: Vec<bool>,
    initial_states: HashSet<StateId>,
}

impl Nfa {

    pub fn new(table: TransitionTable<Transition>, accepting: Vec<bool>, initial_states: HashSet<StateId>) -> Self {
        assert_eq!(table.n_states(), accepting.len());
        Nfa {
            table,
            accepting,
            initial_states,
        }
    }

    pub fn simple_init() -> HashSet<StateId> {
        let mut init = HashSet::new();
        init.insert(0);
        return init
    }

    #[inline]
    pub fn initial_states(&self) -> &HashSet<StateId> {
        &self.initial_states
    }

    pub fn join(&mut self, other: &Nfa) {
        let shift = self.n_states() as StateId;
        self.table.join(&other.table);
        self.initial_states.extend(other.initial_states().iter().map(|s| s + shift));
        self.accepting.extend_from_slice(&other.accepting);
    }

    pub fn add_track(&mut self) {
        self.table = self.table.add_track();
    }

    pub fn determinize(&self) -> Dfa {
        let asize = self.alphabet_size();
        let mut map = HashMap::new();
        let init = self.initial_states.clone();
        map.insert(StateSet::new(init.clone()), 0);

        let mut transitions = vec![0; asize];
        let mut accepting = Vec::new();
        accepting.push(init.iter().any(|s| self.accepting[*s as usize]));

        let mut stack = vec![(0, init)];
        let mut new_id = 0;

        while let Some((s_id, state)) = stack.pop() {
            for a in 0..asize {
                let mut new_state = HashSet::new();
                for s in &state {
                    new_state.extend(self.table.get_transition(*s, a).states.iter());
                }
                let fs = StateSet::new(new_state);
                let id = map.get(&fs).map(|x| *x).unwrap_or_else(|| {
                    let new_state : HashSet<StateId> = fs.inner().clone();
                    new_id += 1;
                    map.insert(fs, new_id);
                    accepting.push(new_state.iter().any(|s| self.accepting[*s as usize]));
                    //transitions.extend_from_slice(&[0; asize]);
                    transitions.resize(transitions.len() + asize, 0 as StateId);
                    stack.push((new_id, new_state));
                    new_id
                });
                /*let id = *map.entry(new_state).or_insert_with(|| {
                    new_id += 1;
                    accepting.push(new_state.iter().any(|s| self.accepting[s]));
                    transitions.extend_from_slice(&[0; asize]);
                    stack.push((new_id, new_state));
                    new_id
                });*/
                transitions[s_id * asize + a] = id as StateId;
            }
        }
        Dfa::new(TransitionTable::new(self.n_tracks(), transitions), accepting)
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
    pub fn n_tracks(&self) -> usize {
        self.table.n_tracks()
    }

    pub fn swap_tracks(&mut self, index1: usize, index2: usize) {
        self.table.swap_tracks(index1, index2);
    }

    pub fn merge_first_track(&mut self) {
        self.table = self.table.merge_first_track()
    }

    pub fn merge_other_tracks(&mut self, track_id: usize) {
        let n_tracks = self.n_tracks();
        assert!(track_id < n_tracks);
        self.swap_tracks(track_id, n_tracks - 1);
        for _ in 0..n_tracks - 1 {
            self.merge_first_track()
        }
    }

    pub fn zero_suffix_closure(&mut self) {
        let mut repeat = true;
        while repeat {
            repeat = false;
            for (i, states) in self.table.states().enumerate() {
                if self.accepting[i] {
                   continue;
                }
                for s in &states[0].states {
                    if self.accepting[*s as usize] {
                        self.accepting[i] = true;
                        repeat = true;
                        break
                    }
                }
            }
        }
    }

    pub fn make_dfa(&self) -> Dfa {
        self.determinize().minimize()
    }

    pub fn write_dot(&self, path: &Path, remove_sink: bool) -> std::io::Result<()> {

        let sink: Option<StateId> = if remove_sink {
            self.accepting.iter().enumerate().find(|(i, a)| {
                !**a && self.table.get_state(*i as StateId).iter().all(|x| x.is_simple(*i as StateId)) && !self.initial_states.contains(&(*i as StateId))
            }).map(|(i, a)| i as StateId)
        } else {
            None
        };

        let mut file = File::create(path)?;
        file.write_all(b"digraph G {\n")?;

        for (i, acc) in self.accepting.iter().enumerate() {
            if Some(i as StateId) == sink {
                continue;
            }
            let shape = if *acc { "doublecircle" } else { "circle"};
            let color = if self.initial_states.contains(&(i as StateId)) { "gray" } else { "none" };
            file.write_all(format!("s{i}[label={i},shape={shape},fillcolor={color}, style=filled]\n", i=i, shape=shape, color=color).as_bytes())?;
        }
        let mut pairs = Vec::new();

        let symbol_strings : Vec<String> = (0..self.table.alphabet_size()).map(|x| format!("{1:00$b}", self.table.n_tracks(), x)).collect();

        for (i, states) in self.table.states().enumerate() {
            if Some(i as StateId) == sink {
                continue;
            }
            pairs.clear();
            for (symbol, target) in states.iter().enumerate() {
                for j in &target.states {
                    if Some(*j) == sink {
                        continue;
                    }
                    pairs.push((*j, symbol));
                }
            }
            pairs.sort();
            for (target, symbols) in &pairs.iter().group_by(|p| p.0) {
                let label = symbols.map(|x| &symbol_strings[x.1]).join(",");
                file.write_all(format!("s{} -> s{} [label=\"{}\"]\n", i, target, label).as_bytes())?;

            };
            //
        }
        file.write_all(b"\n}\n")?;
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn make_nda(n_tracks: usize, tr: Vec<Transition>, acc: Vec<bool>, init: HashSet<StateId>) -> Nfa {
        Nfa::new(TransitionTable::new(n_tracks, tr), acc, init)
    }

    #[test]
    fn test_determinize_simple() {
        /*
            s0 -1-> s1 -1--> s2
            /\
            \/01

         */
        let tr = vec![Transition::simple(0), Transition::pair(0, 1),
                      Transition::empty(), Transition::simple(2),
                      Transition::empty(), Transition::empty()];
        let acc = vec![false, false, true];
        let a = make_nda(1, tr, acc, vec![0].into_iter().collect());

        let m = a.determinize().minimize();
        assert_eq!(m.n_states(), 3);
        assert_eq!(*m.accepting(), vec![false, false, true]);
    }
}