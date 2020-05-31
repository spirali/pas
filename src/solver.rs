use crate::aset::AutomaticSet;
use crate::formula::{LoFormula, LoPredicate};
use crate::parser::{SetDef, Command};
use hashbrown::HashSet;
use crate::name::Name;
use crate::elements::get_max_value;
use crate::render::png::render_set_png;
use std::io::BufWriter;
use std::fs::File;
use std::path::Path;

#[derive(Debug)]
pub struct Context {
    sets: hashbrown::HashMap<Name, AutomaticSet>
}

impl Context {
    pub fn new() -> Self {
        Context {
            sets: Default::default()
        }
    }

    pub fn get_set(&self, name: &Name) -> &AutomaticSet {
        self.sets.get(&name).unwrap_or_else(|| {
           panic!("Set '{:?}' not defined", name);
        })
    }

    pub fn eval(&mut self, cmd: Command) {
        match cmd {
            Command::SetDef(name, setdef) => {
                let name = Name::new(name);
                self.sets.insert(name, build_set(&setdef));
            },
            Command::Call(name, args) => {
                match name.as_str() {
                    "render_png" => {
                        let mut args = args.into_iter();
                        let set_name = Name::new(args.next().unwrap());
                        let output = format!("{}.png", args.next().unwrap());
                        let dfa = self.get_set(&set_name).as_dfa();

                        let file = File::create(output).unwrap();
                        let mut writer = BufWriter::new(file);
                        render_set_png(&[&dfa], &[[255, 0, 0]], &mut writer);
                    },
                    "nfa_dot" => {
                        let mut args = args.into_iter();
                        let set_name = Name::new(args.next().unwrap());
                        let output = format!("{}.dot", args.next().unwrap());
                        let nfa = self.get_set(&set_name).clone().to_nfa();
                        nfa.write_dot(Path::new(&output), true).unwrap();
                    }
                    "stats" => {
                        print_stats(self.get_set(&Name::new(args.into_iter().next().unwrap())))
                    },
                    name => {
                        panic!("Unknown command '{}'", name);
                    }
                }
            }
        }
    }
}

fn print_stats(aset: &AutomaticSet) {
    let names = aset.track_names().to_vec();
    let dfa = aset.as_dfa();
    println!("DFA size: {}", dfa.n_states());
    let nfa = dfa.to_nfa();
    for (i, name) in names.iter().enumerate() {
        println!("Max {:?}: {}", name, get_max_value(&nfa, i).to_string());
    }
}

pub fn evaluate_predicate(pred: &LoPredicate) -> AutomaticSet {
    match pred {
        LoPredicate::EqConst(name, value) => AutomaticSet::singleton(name.clone(), value.clone()),
        LoPredicate::Eq(name1, name2) => AutomaticSet::equivalence(name1.clone(), name2.clone()),
        LoPredicate::Add(name1, name2, name3) => AutomaticSet::addition(name1.clone(), name2.clone(), name3.clone()),
        LoPredicate::Double(name1, name2) => AutomaticSet::double(name1.clone(), name2.clone()),
        LoPredicate::True => AutomaticSet::trivial(true),
        LoPredicate::False => AutomaticSet::trivial(false),
        p => panic!("Not implemented predicate {:?}", p)
    }
}

pub fn evaluate_formula(formula: &LoFormula) -> AutomaticSet {
    match formula {
        LoFormula::Predicate(pred) => evaluate_predicate(pred),
        LoFormula::Or(f1, f2) => evaluate_formula(f1).union(evaluate_formula(f2)),
        LoFormula::Neg(f) => evaluate_formula(f).neg(),
        LoFormula::Exists(name, f) => evaluate_formula(f).exists(name.clone()),
    }
}

pub fn build_set(set_def: &SetDef) -> AutomaticSet {
    /* Check uniqueness of vars */
    let mut uniq = HashSet::new();
    assert!(set_def.vars().iter().all(|x| uniq.insert(x.clone())));

    let formula = set_def.formula().make_lo_formula();
    //dbg!(&formula);
    let mut aset = evaluate_formula(&formula);

    for name in formula.free_vars() {
        if !uniq.contains(&name) {
            aset = aset.exists(name.clone())
        }
    }

    aset.ensure_dfa();
    aset.order_tracks(set_def.vars());
    aset
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_formula;
    use crate::name::Name;

    #[test]
    fn test_eval_eq_formula() {
        let f = parse_formula("x == y").make_lo_formula();
        assert!(evaluate_formula(&f).test_input(&[("x", 10), ("y", 10)]));
        assert!(!evaluate_formula(&f).test_input(&[("x", 10), ("y", 11)]));

        let f = parse_formula("x == x").make_lo_formula();
        assert!(evaluate_formula(&f).test_input(&[("x", 10)]));
        assert!(evaluate_formula(&f).test_input(&[("x", 11)]));

        let f = parse_formula("x == y and y == z").make_lo_formula();
        assert!(evaluate_formula(&f).test_input(&[("x", 10), ("y", 10), ("z", 10)]));
        assert!(!evaluate_formula(&f).test_input(&[("x", 11), ("y", 10), ("z", 10)]));
        assert!(!evaluate_formula(&f).test_input(&[("x", 9), ("y", 10), ("z", 9)]));
        assert!(evaluate_formula(&f).test_input(&[("x", 0), ("y", 0), ("z", 0)]));

        let f = parse_formula("x == y or y == z").make_lo_formula();
        assert!(evaluate_formula(&f).test_input(&[("x", 10), ("y", 10), ("z", 10)]));
        assert!(evaluate_formula(&f).test_input(&[("x", 10), ("y", 10), ("z", 0)]));
        assert!(evaluate_formula(&f).test_input(&[("x", 10), ("y", 0), ("z", 0)]));
        assert!(!evaluate_formula(&f).test_input(&[("x", 0), ("y", 10), ("z", 0)]));
    }

    #[test]
    fn test_eval_simple_plus_formula() {
        let f = parse_formula("x + y == z").make_lo_formula();
        let g = parse_formula("x == y + z").make_lo_formula();
        assert!(evaluate_formula(&f).test_input(&[("x", 4), ("y", 6), ("z", 10)]));

        let mut a = evaluate_formula(&f);
        let mut b = evaluate_formula(&g);
        for i in &[0, 1, 2, 3, 17, 37, 100, 111, 317, 255, 256] {
            for j in 0..100 {
                assert!(a.test_input(&[("x", *i), ("y", j), ("z", i + j)]));
                assert!(!a.test_input(&[("x", *i + 1), ("y", j), ("z", i + j)]));
                assert!(!a.test_input(&[("x", *i), ("y", j + 1), ("z", i + j)]));
                assert!(b.test_input(&[("x", i + j), ("y", j), ("z", *i)]));
                assert!(!b.test_input(&[("x", *i + 1), ("y", j), ("z", i + j)]));
                assert!(!b.test_input(&[("x", *i), ("y", j + 1), ("z", i + j)]));
                if *i > 0 && j > 0 {
                    assert!(!b.test_input(&[("x", *i), ("y", j), ("z", i + j / 2)]));
                }
            }
        }
    }

    #[test]
    fn test_eval_eq_const_formula() {
        let mut a = evaluate_formula(&parse_formula("x == 5 or x == 7").make_lo_formula());
        assert!(a.test_input(&[("x", 5)]));
        assert!(a.test_input(&[("x", 7)]));
        assert!(!a.test_input(&[("x", 6)]));
        assert!(!a.test_input(&[("x", 10)]));
    }

    #[test]
    fn test_eval_combined_plus_formula() {
        let mut a = evaluate_formula(&parse_formula("x + y + z == w").make_lo_formula());
        assert!(a.test_input(&[("x", 1), ("y", 2), ("z", 3), ("w", 6)]));
        assert!(!a.test_input(&[("x", 1), ("y", 2), ("z", 3), ("w", 7)]));

        let mut a = evaluate_formula(&parse_formula("x + y + z == v + w + x").make_lo_formula());
        assert!(a.test_input(&[("x", 1), ("y", 2), ("z", 3), ("w", 1), ("v", 4)]));
        assert!(a.test_input(&[("x", 0), ("y", 2), ("z", 3), ("w", 1), ("v", 4)]));
        assert!(!a.test_input(&[("x", 0), ("y", 6), ("z", 3), ("w", 1), ("v", 4)]));
        assert!(!a.test_input(&[("x", 1), ("y", 2), ("z", 3), ("w", 1), ("v", 3)]));


        let mut a = evaluate_formula(&parse_formula("x + 2 == y + 3").make_lo_formula());
        assert!(a.test_input(&[("x", 2), ("y", 1)]));
        assert!(!a.test_input(&[("x", 1), ("y", 2)]));

        let mut a = evaluate_formula(&parse_formula("x + y + z + 2 + 7 == v + w + x + 3").make_lo_formula());
        //a.clone().to_dfa().to_nfa().write_dot(std::path::Path::new("/tmp/xx.dot")).unwrap();
        assert!(a.test_input(&[("x", 1), ("y", 2), ("z", 3), ("w", 7), ("v", 4)]));
    }

    #[test]
    fn test_eval_combined_lt_formula() {
        let mut a = evaluate_formula(&parse_formula("x < 10").make_lo_formula());
        for i in 0..10 {
            assert!(a.test_input(&[("x", i)]));
        }
        for i in 10..20 {
            assert!(!a.test_input(&[("x", i)]));
        }
    }

    #[test]
    fn test_eval_mul_formula() {
        let mut a = evaluate_formula(&parse_formula("2 * x < 10").make_lo_formula());
        for i in 0..5 {
            assert!(a.test_input(&[("x", i)]));
        }
        for i in 6..20 {
            assert!(!a.test_input(&[("x", i)]));
        }

        let mut a = evaluate_formula(&parse_formula("3 * x == 60").make_lo_formula());
        for i in 0..100 {
            assert_eq!(a.test_input(&[("x", i)]), i == 20);
        }

        //let mut a = evaluate_formula(&parse_formula("1325 * x == 147075").make_lo_formula());

        let f = parse_formula("11 * x == 3 * y").make_lo_formula();
        let mut a = evaluate_formula(&f);
        assert!(a.test_input(&[("x", 0), ("y", 0)]));
        assert!(a.test_input(&[("x", 3), ("y", 11)]));



        let mut a = evaluate_formula(&f);
        /*a.clone().to_dfa().to_nfa().write_dot(std::path::Path::new("/tmp/x.dot"), true).unwrap();
        iterate_words(&a.clone().to_dfa(), Some(10), |w|  println!("WW {:?}", w));*/

        let f = parse_formula("111 * x == 30 * y").make_lo_formula();
        let mut a = evaluate_formula(&f);
        assert!(a.test_input(&[("x", 30), ("y", 111)]));
        assert!(!a.test_input(&[("x", 31), ("y", 111)]));
        assert!(!a.test_input(&[("x", 30), ("y", 110)]));

        let mut a = evaluate_formula(&parse_formula("x % 7 == 2 and x > 100").make_lo_formula());
        for i in 0..200 {
            assert_eq!(a.test_input(&[("x", i)]), i % 7 == 2 && i > 100);
        }
    }

    #[test]
    fn test_eval_combined_lte_formula() {
        let mut a = evaluate_formula(&parse_formula("x <= 10").make_lo_formula());
        for i in 0..11 {
            assert!(a.test_input(&[("x", i)]));
        }
        for i in 11..20 {
            assert!(!a.test_input(&[("x", i)]));
        }
    }

    #[test]
    fn test_eval_is_empty() {
        let mut a = evaluate_formula(&parse_formula("x < 10 and x > 10").make_lo_formula());
        assert!(a.is_empty());
    }

    #[test]
    fn test_eval_is_not_empty() {
        let mut a = evaluate_formula(&parse_formula("x < 10 and x > 5").make_lo_formula());
        assert!(!a.is_empty());
    }
}
