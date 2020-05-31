use std::cell::Cell;
use nom::lib::std::fmt::Formatter;
use std::fmt;

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Name {
    Named(String),
    Unnamed(usize),
    Tmp(usize),
}

thread_local!(
    static ID_COUNTER: Cell<usize> = Cell::new(0);
);

impl Name {
    pub fn new(string: String) -> Self {
        Self::Named(string)
    }

    pub fn from_str(str: &str) -> Self {
        Self::Named(str.to_string())
    }

    pub fn new_tmp() -> Self {
        let n = ID_COUNTER.with(|cell| {
            let n = cell.get();
            cell.set(n + 1);
            n
        });
        Name::Tmp(n)
    }

    pub fn new_unnamed() -> Self {
        let n = ID_COUNTER.with(|cell| {
            let n = cell.get();
            cell.set(n + 1);
            n
        });
        Name::Unnamed(n)
    }

    #[inline]
    pub fn is_tmp(&self) -> bool {
        match self {
           Self::Tmp(_) => true,
           _ => false,
        }
    }
}

impl fmt::Debug for Name {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&match self {
               Self::Named(s) => format!("{}", &s),
               Self::Unnamed(s) => format!("${}", &s),
               Self::Tmp(s) => format!("#{}", &s),
        })
    }
}