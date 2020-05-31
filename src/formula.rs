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
            },
            Self::Exists(name, f) => {
                let mut vars = f.free_vars();
                vars.remove(&name);
                vars
            },
        }
    }

    pub fn rename_free_var(self, name_from: &Name, name_to: &Name) -> Self {
        match self {
            Self::Predicate(p) => Self::Predicate(p.rename_free_var(name_from, name_to)),
            Self::Neg(f) => Self::Neg(Box::new(f.rename_free_var(name_from, name_to))),
            Self::Or(f1, f2) => {
                Self::Or(Box::new(f1.rename_free_var(name_from, name_to)), Box::new(f2.rename_free_var(name_from, name_to)))
            },
            Self::Exists(name, f) if &name != name_from => Self::Exists(name, Box::new(f.rename_free_var(name_from, name_to))),
            x @ Self::Exists(_, _) => x,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Variable {
    pub(crate) name: Name,
    scale: u64,
    modulo: u64,
}

impl Variable {
    pub fn from_str(name: &str) -> Self {
        Variable {
            name: Name::from_str(name),
            scale: 1,
            modulo: 1,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Expression {
    Variable(Name),
    Constant(u64),
    Add(Vec<Expression>),
    Mul(Box<Expression>, u64),
    Mod(Box<Expression>, u64),
}

impl Expression {
    pub fn from_name(name: Name) -> Self {
        Self::Variable(name)
    }

    pub fn new_add(exprs: Vec<Expression>) -> Self {
        assert!(!exprs.is_empty());
        if exprs.len() == 1 {
            return exprs.into_iter().next().unwrap();
        }
        Expression::Add(exprs)
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
    BinOp(BinOp, Expression, Expression),
    True,
    False
}

impl HiPredicate {

    fn eq_optimize(lhs: &Expression, rhs: &Expression) -> Option<LoFormula> {
        match (lhs, rhs) {
            /*([Expression::Variable(name1), Expression::Variable(name2)], [Expression::Variable(name3)]) => {
                Some(LoPredicate::safe_add(name1.clone(), name2.clone(), name3.clone()).to_formula())
            },*/
            /*([Expression::Variable(Variable{name: name1, scale: 2, modulo: 1})], [Expression::Variable(Variable{name: name2, scale: 1, modulo: 1})]) => {
                Some(LoPredicate::safe_double(name1.clone(), name2.clone()).to_formula())
            },*/
            (Expression::Variable(name), Expression::Constant(c)) => {
                Some(LoPredicate::EqConst(name.clone(), *c).to_formula())
            },
            _ => None,
        }
    }

    pub fn make_lo_formula(&self) -> LoFormula {
        match self {
            HiPredicate::BinOp(op, lhs, rhs) => {
                if let BinOp::Eq = op {
                    if let Some(f) = Self::eq_optimize(lhs, rhs) {
                        return f;
                    }
                    if let Some(f) = Self::eq_optimize(rhs, lhs) {
                        return f;
                    }
                }
                let (lf, name1) = Self::expression_to_lo_formula(lhs);
                let (rf, name2) = Self::expression_to_lo_formula(rhs);
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

    fn expression_to_lo_formula(expression: &Expression) -> (LoFormula, Name) {
        match expression {
            Expression::Constant(v) => {
                let fresh = Name::new_tmp();
                (LoPredicate::EqConst(fresh.clone(), *v).to_formula(), fresh)
            },
            Expression::Variable(v) => (LoPredicate::True.to_formula(), v.clone()),
            Expression::Add(es) => {
                assert!(es.len() >= 2);
                let (mut f1, mut name1) = Self::expression_to_lo_formula(&es[0]);
                for e in &es[1..] {
                    let (f2, name2) = Self::expression_to_lo_formula(e);
                    let fresh = Name::new_tmp();
                    let f3 = LoFormula::Predicate(LoPredicate::safe_add(name1.clone(), name2.clone(), fresh.clone()));
                    f1 = f3.and(f1).close_if_tmp(&name1).and(f2).close_if_tmp(&name2);
                    name1 = fresh
                }
                (f1, name1)
            },
            Expression::Mod(expr, x) => {
                // E % x == OUT ~~> exists(T)(T * x + OUT == E) and OUT < x
                let var_t = Name::new_unnamed();
                let var_out = Name::new_unnamed();
                let tmp_e1 = Expression::Add(vec![Expression::Mul(Box::new(Expression::Variable(var_t.clone())), *x), Expression::Variable(var_out.clone())]);
                let formula1 = HiFormula::Exists(var_t.clone(), Box::new(HiFormula::Predicate(HiPredicate::BinOp(BinOp::Eq, *expr.clone(), tmp_e1))));
                let formula2 = HiFormula::Predicate(HiPredicate::BinOp(BinOp::Lt, Expression::Variable(var_out.clone()), Expression::Constant(*x)));
                let formula = HiFormula::And(Box::new(formula1), Box::new(formula2));
                let fresh_tmp = Name::new_tmp();
                (formula.make_lo_formula().rename_free_var(&var_out, &fresh_tmp), fresh_tmp)

            },
            Expression::Mul(e, mut x) => {
                if x == 0 {
                    todo!();
                }
                let (mut formula, mut exp_var) = Self::expression_to_lo_formula(e);
                let mut out_var = None;
                //let mut formula = LoPredicate::True.to_formula();
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