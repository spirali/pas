use std::cell::Cell;
use nom::lib::std::fmt::Formatter;
use std::fmt;

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Name {
    Named(String),
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

    #[inline]
    pub fn is_tmp(&self) -> bool {
        match self {
           Self::Named(_) => false,
           Self::Tmp(_) => true,
        }
    }
}

impl fmt::Debug for Name {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&match self {
               Self::Named(s) => format!("{}", &s),
               Self::Tmp(s) => format!("#{}", &s),
        })
    }
}