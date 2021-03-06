use super::{Dfa, Nfa};

#[derive(Debug, Clone)]
pub enum Automaton {
    Dfa(Dfa),
    Nfa(Nfa),
}

impl Automaton {
    pub fn into_nfa(self) -> Nfa {
        match self {
            Self::Dfa(dfa) => dfa.to_nfa(),
            Self::Nfa(nfa) => nfa,
        }
    }

    pub fn into_dfa(self) -> Dfa {
        match self {
            Self::Dfa(dfa) => dfa,
            Self::Nfa(nfa) => nfa.make_dfa(),
        }
    }

    pub fn make_dfa(&self) -> Dfa {
        match self {
            Self::Dfa(dfa) => dfa.clone(),
            Self::Nfa(nfa) => nfa.make_dfa(),
        }
    }

    pub fn add_track(&mut self) {
        match self {
            Self::Dfa(dfa) => dfa.add_track(),
            Self::Nfa(nfa) => nfa.add_track(),
        }
    }

    pub fn swap_tracks(&mut self, index1: usize, index2: usize) {
        match self {
            Self::Dfa(dfa) => dfa.swap_tracks(index1, index2),
            Self::Nfa(nfa) => nfa.swap_tracks(index1, index2),
        }
    }

    pub fn ensure_dfa(&mut self) -> &Dfa {
        match self {
            Self::Dfa(dfa) => dfa,
            Self::Nfa(nfa) => {
                let dfa = Self::Dfa(nfa.determinize().minimize());
                *self = dfa;
                self.ensure_dfa()
            }
        }
    }

    pub fn alphabet_size(&self) -> usize {
        match self {
            Self::Dfa(dfa) => dfa.alphabet_size(),
            Self::Nfa(nfa) => nfa.alphabet_size(),
        }
    }
}