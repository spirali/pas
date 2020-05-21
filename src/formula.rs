use crate::name::{Name};
use hashbrown::HashSet;
use crate::formula::LoPredicate::EqConst;

#[derive(Debug)]
pub enum LoPredicate {
    Add(Name, Name, Name), // x + y = z
    Double(Name, Name),    // 2 * x = y
    Eq(Name, Name),        // x == y
    EqConst(Name, u64),    // x == C
    True,
    False
}

impl LoPredicate {

    pub fn free_vars(&self) -> HashSet<Name> {
        let mut out = HashSet::new();
        match self {
            Self::Add(name1, name2, name3) => {
                out.insert(name1.clone());
                out.insert(name2.clone());
                out.insert(name3.clone());
            },
            Self::Eq(name1, name2) | Self::Double(name1, name2) => {
                out.insert(name1.clone());
                out.insert(name2.clone());
            },
            Self::EqConst(name1, _) => {
                out.insert(name1.clone());
            },
            Self::True | Self::False => { /* Do nothing */ },
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
            },
            Self::Exists(name, f) => {
                let mut vars = f.free_vars();
                vars.remove(&name);
                vars
            },
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Atom {
    Variable(Name, u64),
    Constant(u64),
}

impl Atom {
    pub fn from_name(name: Name) -> Self {
        Self::Variable(name, 1)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum BinOp {
    Eq,
    Lt,
    Lte,
}

#[derive(Debug, Eq, PartialEq)]
pub enum HiPredicate {
    BinOp(BinOp, Vec<Atom>, Vec<Atom>),
    True,
    False
}

impl HiPredicate {

    fn eq_optimize(lhs: &[Atom], rhs: &[Atom]) -> Option<LoFormula> {
        match (lhs, rhs) {
            ([Atom::Variable(name1, 1), Atom::Variable(name2, 1)], [Atom::Variable(name3, 1)]) => {
                Some(LoPredicate::safe_add(name1.clone(), name2.clone(), name3.clone()).to_formula())
            },
            ([Atom::Variable(name1, 2)], [Atom::Variable(name2, 1)]) => {
                Some(LoPredicate::safe_double(name1.clone(), name2.clone()).to_formula())
            },
            ([Atom::Variable(name, 1)], [Atom::Constant(c)]) => {
                Some(LoPredicate::EqConst(name.clone(), *c).to_formula())
            },
            _ => None,
        }
    }

    pub fn make_lo_formula(&self) -> LoFormula {
        match self {
            HiPredicate::BinOp(op, lhs, rhs) => {
                if let BinOp::Eq = op {
                    if let Some(f) = Self::eq_optimize(lhs.as_slice(), rhs.as_slice()) {
                        return f;
                    }
                    if let Some(f) = Self::eq_optimize(rhs.as_slice(), lhs.as_slice()) {
                        return f;
                    }
                }
                let (lf, name1) = Self::expr_to_lo_formula(&lhs);
                let (rf, name2) = Self::expr_to_lo_formula(&rhs);
                if name1 == name2 {
                    match op {
                        BinOp::Lt => LoPredicate::False.to_formula(),
                        BinOp::Eq | BinOp::Lte => LoPredicate::True.to_formula(),
                    }
                } else {
                    let f = match op {
                        BinOp::Eq => LoPredicate::safe_eq(name1.clone(), name2.clone()).to_formula(),
                        BinOp::Lte => {
                            let fresh = Name::new_tmp();
                            LoPredicate::safe_add(name1.clone(), fresh.clone(), name2.clone()).to_formula().exists(fresh)
                        }
                        BinOp::Lt => {
                            let fresh = Name::new_tmp();
                            LoPredicate::safe_add(name2.clone(), fresh.clone(), name1.clone()).to_formula().exists(fresh).neg()
                        }
                    };
                    f.and(lf).close_if_tmp(&name1).and(rf).close_if_tmp(&name2)
                }
            },
            HiPredicate::True => LoFormula::Predicate(LoPredicate::True),
            HiPredicate::False => LoFormula::Predicate(LoPredicate::False),
            _ => unimplemented!(),
        }
    }

    fn atom_to_lo_formula(atom: &Atom) -> (LoFormula, Name) {
        match atom {
            Atom::Constant(v) => {
                let fresh = Name::new_tmp();
                (LoPredicate::EqConst(fresh.clone(), *v).to_formula(), fresh)
            },
            Atom::Variable(v, 1) => (LoPredicate::True.to_formula(), v.clone()),
            Atom::Variable(v, 0) => (LoPredicate::EqConst(v.clone(), 0).to_formula(), v.clone()),
            Atom::Variable(v, mut x) => {
                //let mut value = x;
                let mut exp_var = v.clone();
                let mut out_var = None;
                let mut formula = LoPredicate::True.to_formula();
                loop {
                    let mut close = true;
                    if x & 1 == 1 {
                        if out_var.is_none() {
                            out_var = Some(exp_var.clone());
                            close = false;
                        } else {
                            let fresh2 = Name::new_tmp();
                            let var = out_var.take().unwrap();
                            formula = formula.and(LoPredicate::safe_add(var.clone(), exp_var.clone(), fresh2.clone()).to_formula()).close_if_tmp(&var);
                            out_var = Some(fresh2);
                        }
                    }
                    x >>= 1;
                    if x == 0 {
                        if close {
                            formula = formula.close_if_tmp(&exp_var);
                        }
                        return (formula, out_var.unwrap());
                    }
                    let mut fresh = Name::new_tmp();
                    formula = formula.and(LoPredicate::safe_double(exp_var.clone(), fresh.clone()).to_formula());
                    if close {
                        formula = formula.close_if_tmp(&exp_var);
                    }
                    exp_var = fresh;
                }
            }
        }
    }

    fn expr_to_lo_formula(atoms: &[Atom]) -> (LoFormula, Name) {
        // x + y + z = fresh1 ===>  x + y
        if atoms.len() == 1 {
            return Self::atom_to_lo_formula(&atoms[0]);
        }
        assert!(atoms.len() > 1);
        let fresh = Name::new_tmp();
        let (f1, name1) = Self::atom_to_lo_formula(&atoms[0]);
        let (f2, name2) = Self::expr_to_lo_formula(&atoms[1..]);
        let f3 = LoFormula::Predicate(LoPredicate::safe_add(name1.clone(), name2.clone(), fresh.clone()));
        (f3.and(f1).close_if_tmp(&name1).and(f2).close_if_tmp(&name2), fresh)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum HiFormula {
    Predicate(HiPredicate),
    Neg(Box<HiFormula>),
    And(Box<HiFormula>, Box<HiFormula>),
    Or(Box<HiFormula>, Box<HiFormula>),
    Exists(Name, Box<HiFormula>),
    ForAll(Name, Box<HiFormula>),
}

impl HiFormula {

    pub fn make_lo_formula(&self) -> LoFormula {
        match self {
            HiFormula::Predicate(p) => p.make_lo_formula(),
            HiFormula::Neg(f) => f.make_lo_formula().neg(),
            HiFormula::And(f1, f2) => f1.make_lo_formula().and(f2.make_lo_formula()),
            HiFormula::Or(f1, f2) => f1.make_lo_formula().or(f2.make_lo_formula()),
            HiFormula::Exists(name, f) => f.make_lo_formula().exists(name.clone()),
            HiFormula::ForAll(name, f) => f.make_lo_formula().for_all(name.clone()),
        }
    }

    pub fn and(self, other: HiFormula) -> HiFormula {
        HiFormula::And(Box::new(self), Box::new(other))
    }

    pub fn or(self, other: HiFormula) -> HiFormula {
        HiFormula::Or(Box::new(self), Box::new(other))
    }

    pub fn neg(self) -> HiFormula {
        HiFormula::Neg(Box::new(self))
    }
}