use std::hash::{Hasher, Hash};
use hashbrown::HashSet;


pub type StateId = u32;

#[derive(Debug)]
pub struct StateSet {
    inner: HashSet<StateId>
}

impl StateSet {
    pub fn new(set: HashSet<StateId>) -> Self {
        StateSet { inner: set }
    }

    #[inline]
    pub fn inner(&self) -> &HashSet<StateId> {
        &self.inner
    }
}

impl PartialEq for StateSet
{
    fn eq(&self, other: &StateSet) -> bool {
        self.inner == other.inner
    }
}

impl Eq for StateSet {}

impl Hash for StateSet {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        let mut h : StateId = 0;
        for elm in &self.inner {
            h ^= *elm;
        }
        state.write_u64(h as u64 * self.inner.len() as u64);
    }
}

/*
pub struct FrozenSet<T> {
    inner: HashSet<T>
}

impl<T> FrozenSet<T> {
    pub fn new(set: HashSet<T>) -> Self {
        FrozenSet{ inner: set }
    }

    pub fn inner(&self) -> &HashSet<T> {
        &self.inner
    }
}

impl<T> PartialEq for FrozenSet<T>
    where
    T: Eq + Hash
{
    fn eq(&self, other: &FrozenSet<T>) -> bool {
        self.inner == other.inner
    }
}

impl<T> Eq for FrozenSet<T> where T: Eq + Hash {}

impl<T> Hash for FrozenSet<T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        let h = self.inner.len() as u64;
        for elm in &self.inner {
            let mut hasher = DefaultHasher::new();
            elm.hash(&mut hasher);
            h ^= hasher.finish();
        }
        state.write_u64(h);
    }
}

 */

/*#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Copy)]
pub enum Value {
    Finite(usize),
    Infinite,
}

impl Value {

    #[inline]
    pub fn as_usize(&self) -> Option<usize> {
        match self {
            Self::Finite(x) => Some(*x),
            Self::Infinite => None
        }
    }
}*/