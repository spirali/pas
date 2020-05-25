
use crate::common::StateId;
use crate::nfa::Transition;

#[derive(Debug, Clone)]
pub struct TransitionTable<T : Default + Clone> {
    n_tracks: usize,
    transitions: Vec<T>
}


impl<T : Default + Clone> TransitionTable<T> {
    pub fn new(n_tracks: usize, transitions: Vec<T>) -> Self {
        assert_eq!(transitions.len() % (1 << n_tracks), 0);
        TransitionTable {
            n_tracks,
            transitions,
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.transitions.len()
    }

    #[inline]
    pub fn get_transition(&self, state: StateId, symbol: usize) -> &T {
        &self.transitions[state as usize * self.alphabet_size() + symbol]
    }

    #[inline]
    pub fn alphabet_size(&self) -> usize {
        1 << self.n_tracks
    }

    #[inline]
    pub fn n_tracks(&self) -> usize {
        self.n_tracks
    }

    #[inline]
    pub fn n_states(&self) -> usize {
        self.transitions.len() / self.alphabet_size()
    }

    pub fn as_slice(&self) -> &[T] {
        &self.transitions
    }

    #[inline]
    pub fn get_state_mut(&mut self, state: StateId) -> &mut [T] {
        let asize = self.alphabet_size();
        let start = state as usize * asize;
        &mut self.transitions[start..start + asize]
    }

    #[inline]
    pub fn get_state(&self, state: StateId) -> &[T] {
        let asize = self.alphabet_size();
        let start = state as usize * asize;
        &self.transitions[start..start + asize]
    }

    pub fn states_mut(&mut self) -> impl Iterator<Item=&mut [T]> {
        let size = self.alphabet_size();
        self.transitions.chunks_mut(size)
    }

    pub fn states(&self) -> impl Iterator<Item=&[T]> {
        let size = self.alphabet_size();
        self.transitions.chunks(size)
    }

    pub fn map_states<S, F>(&self, f: F) -> TransitionTable<S> where F: FnMut(&T) -> S, S: Default + Clone {
        TransitionTable {
            n_tracks: self.n_tracks,
            transitions: self.transitions.iter().map(f).collect(),
        }
    }

    pub fn swap_tracks(&mut self, track1: usize, track2: usize) {
        assert!(track1 < self.n_tracks);
        assert!(track2 < self.n_tracks);
        if track1 == track2 {
            return;
        }
        let asize = self.alphabet_size();
        let mask1 = 1 << track1;
        let mask2 = 1 << track2;
        let mask = mask1 | mask2;
        for row in self.states_mut() {
            for a in 0..asize {
                if (a & mask1 == 0) & (a & mask2 > 0) {
                    let tmp = std::mem::take(&mut row[a]);
                    row[a] = std::mem::take(&mut row[a ^ mask]);
                    row[a ^ mask] = tmp;
                }
            }
        }
    }

    pub fn add_track(&self) -> TransitionTable<T> {
        let mut new_transitions = Vec::with_capacity(self.n_states() * self.alphabet_size() * 2);
        for state in self.states() {
            new_transitions.extend_from_slice(state);
            new_transitions.extend_from_slice(state);
        }
        TransitionTable::new(self.n_tracks + 1, new_transitions)
    }
}

impl TransitionTable<StateId> {
    pub fn fill_partitions(&self, partitions: &[StateId], out: &mut [StateId]) {
        assert_eq!(out.len(), self.transitions.len());
        for (idx, s_id) in self.transitions.iter().enumerate() {
            out[idx] = partitions[*s_id as usize];
        }
    }
}

impl TransitionTable<Transition> {

    pub fn merge_first_track(&self) -> TransitionTable<Transition> {
        assert!(self.n_tracks > 0);
        let mut new_transitions = Vec::new();
        let target_asize = self.alphabet_size() / 2;
        new_transitions.resize(target_asize * self.n_states(), Transition::empty());

        for (idx, state) in self.states().enumerate() {
            for (eidx, e) in state.iter().enumerate() {
                let tr = &mut new_transitions[idx * target_asize + eidx / 2];
                for s in &e.states {
                    if !tr.states.contains(&s) {
                        tr.states.push(*s);
                    }
                }
            }
        }
        TransitionTable::new(self.n_tracks - 1, new_transitions)
    }

    pub fn join(&mut self, other: &TransitionTable<Transition>) {
        assert_eq!(self.n_tracks, other.n_tracks());
        let shift = self.n_states() as StateId;
        self.transitions.extend(other.transitions.iter().map(|t| Transition::new(t.states.iter().map(|s| s + shift).collect())));
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_switch() {
        let data = vec![
            1,2,3,4,5,6,7,8,
            9,10,11,12,13,14,15,16,
            17,18,19,20,21,22,23,24,
        ];
        let mut table = TransitionTable::new(3, data.clone());
        table.swap_tracks(1, 1);
        assert_eq!(table.transitions, data);
        /*
            0 0 0 | 0
            0 0 1 | 1
            0 1 0 | 2
            0 1 1 | 3
            1 0 0 | 4
            1 0 1 | 5
            1 1 0 | 6
            1 1 1 | 7
         */
        let data2 = vec![
            1,3,2,4,5,7,6,8,
            9,11,10,12,13,15,14,16,
            17,19,18,20,21,23,22,24,
        ];
        table.swap_tracks(0, 1);
        assert_eq!(table.transitions, data2);

        let data = vec![
            1,2,3,4,5,6,7,8,
        ];
        let data3 = vec![
            1,2,5,6,3,4,7,8,
        ];
        let mut table = TransitionTable::new(3, data.clone());
        table.swap_tracks(1, 2);
        assert_eq!(table.transitions, data3);

        let data3 = vec![
            1,5,3,7,2,6,4,8,
        ];
        let mut table = TransitionTable::new(3, data.clone());
        table.swap_tracks(2, 0);
        assert_eq!(table.transitions, data3);
    }

    #[test]
    fn test_table_add_track() {
        let data = vec![
            0, 1, 2, 3,
            4, 5, 6, 7
        ];
        let table = TransitionTable::new(2, data);
        let new_table = table.add_track();
        assert_eq!(3, new_table.n_tracks);
        assert_eq!(new_table.transitions, vec![
            0, 1, 2, 3, 0, 1, 2, 3,
            4, 5, 6, 7, 4, 5, 6, 7,
        ]);
    }
}