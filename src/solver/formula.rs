use hashbrown::HashSet;

use crate::common::Name;

#[derive(Debug)]
pub enum LoPredicate {
    Add(Name, Name, Name),
    // x + y = z
    Double(Name, Name),
    // 2 * x = y
    Eq(Name, Name),
    // x == y
    EqConst(Name, u64),
    // x == C
    True,
    False,
}

impl LoPredicate {
    pub fn free_vars(&self) -> HashSet<Name> {
        let mut out = HashSet::new();
        match self {
            Self::Add(name1, name2, name3) => {
                out.insert(name1.clone());
                out.insert(name2.clone());
                out.insert(name3.clone());
            }
            Self::Eq(name1, name2) | Self::Double(name1, name2) => {
                out.insert(name1.clone());
                out.insert(name2.clone());
            }
            Self::EqConst(name1, _) => {
                out.insert(name1.clone());
            }
            Self::True | Self::False => { /* Do nothing */ }
        };
        out
    }

    pub fn safe_eq(name1: Name, name2: Name) -> LoPredicate {
        if name1 == name2 {
            LoPredicate::True
        } else {
            LoPredicate::Eq(name1, name2)
        }
    }

    pub fn safe_add(name1: Name, name2: Name, name3: Name) -> LoPredicate {
        if name1 == name3 {
            LoPredicate::EqConst(name2, 0)
        } else if name2 == name3 {
            LoPredicate::EqConst(name1, 0)
        } else if name1 == name2 {
            LoPredicate::Double(name1, name2)
        } else {
            LoPredicate::Add(name1, name2, name3)
        }
    }

    pub fn safe_double(name1: Name, name2: Name) -> LoPredicate {
        if name1 == name2 {
            LoPredicate::EqConst(name2, 0)
        } else {
            LoPredicate::Double(name1, name2)
        }
    }

    pub fn to_formula(self) -> LoFormula {
        LoFormula::Predicate(self)
    }

    pub fn rename_free_var(self, name_from: &Name, name_to: &Name) -> Self {
        let change = |name| if name_from == &name { name_to.clone() } else { name };
        match self {
            Self::Add(name1, name2, name3) => Self::Add(change(name1), change(name2), change(name3)),
            Self::Eq(name1, name2) => Self::Eq(change(name1), change(name2)),
            Self::Double(name1, name2) => Self::Double(change(name1), change(name2)),
            Self::EqConst(name1, v) => Self::EqConst(change(name1), v),
            Self::True | Self::False => self,
        }
    }
}

#[derive(Debug)]
pub enum LoFormula {
    Predicate(LoPredicate),
    Neg(Box<LoFormula>),
    Or(Box<LoFormula>, Box<LoFormula>),
    Exists(Name, Box<LoFormula>),
}

impl LoFormula {
    pub fn neg(self) -> LoFormula {
        match self {
            LoFormula::Neg(x) => *x,
            LoFormula::Predicate(LoPredicate::True) => LoFormula::Predicate(LoPredicate::False),
            LoFormula::Predicate(LoPredicate::False) => LoFormula::Predicate(LoPredicate::True),
            x => LoFormula::Neg(Box::new(x))
        }
    }

    pub fn or(self, other: LoFormula) -> LoFormula {
        match (self, other) {
            (LoFormula::Predicate(LoPredicate::True), _) => LoFormula::Predicate(LoPredicate::True),
            (_, LoFormula::Predicate(LoPredicate::True)) => LoFormula::Predicate(LoPredicate::True),
            (LoFormula::Predicate(LoPredicate::False), x) => x,
            (x, LoFormula::Predicate(LoPredicate::False)) => x,
            (x, y) => LoFormula::Or(Box::new(x), Box::new(y))
        }
    }

    pub fn and(self, other: LoFormula) -> LoFormula {
        self.neg().or(other.neg()).neg()
    }

    pub fn exists(self, name: Name) -> LoFormula {
        LoFormula::Exists(name, Box::new(self))
    }

    pub fn close_if_tmp(self, name: &Name) -> LoFormula {
        if name.is_tmp() {
            self.exists(name.clone())
        } else {
            self
        }
    }

    pub fn for_all(self, name: Name) -> LoFormula {
        self.neg().exists(name).neg()
    }

    pub fn size(&self) -> usize {
        match self {
            Self::Predicate(p) => 1,
            Self::Neg(f) | Self::Exists(_, f) => f.size() + 1,
            Self::Or(f1, f2) => f1.size() + f2.size() + 1,
        }
    }

    pub fn depth(&self) -> usize {
        match self {
            Self::Predicate(p) => 1,
            Self::Neg(f) | Self::Exists(_, f) => f.depth() + 1,
            Self::Or(f1, f2) => f1.depth().max(f2.depth()) + 1,
        }
    }

    pub fn free_vars(self) -> HashSet<Name> {
        match self {
            Self::Predicate(p) => p.free_vars(),
            Self::Neg(f) => f.free_vars(),
            Self::Or(f1, f2) => {
                let mut vars = f1.free_vars();
                vars.extend(f2.free_vars());
                vars
            }
            Self::Exists(name, f) => {
                let mut vars = f.free_vars();
                vars.remove(&name);
                vars
            }
        }
    }

    pub fn rename_free_var(self, name_from: &Name, name_to: &Name) -> Self {
        match self {
            Self::Predicate(p) => Self::Predicate(p.rename_free_var(name_from, name_to)),
            Self::Neg(f) => Self::Neg(Box::new(f.rename_free_var(name_from, name_to))),
            Self::Or(f1, f2) => {
                Self::Or(Box::new(f1.rename_free_var(name_from, name_to)), Box::new(f2.rename_free_var(name_from, name_to)))
            }
            Self::Exists(name, f) if &name != name_from => Self::Exists(name, Box::new(f.rename_free_var(name_from, name_to))),
            x @ Self::Exists(_, _) => x,
        }
    }
}