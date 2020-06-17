use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use hashbrown::HashSet;

use crate::common::Name;
use crate::highlevel::hiformula::HiFormula;
use crate::render::png::render_set_png;
use crate::solver::{AutomaticSet, get_max_value};
use crate::solver::evaluate_formula;

#[derive(Debug)]
pub struct SetDef {
    pub vars: Vec<Name>,
    pub formula: HiFormula,
}

impl SetDef {
    pub fn vars(&self) -> &[Name] {
        &self.vars
    }

    pub fn formula(&self) -> &HiFormula {
        &self.formula
    }
}


#[derive(Debug)]
pub enum Command {
    SetDef(String, SetDef),
    Call(String, Vec<String>),
}


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
            }
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
                    }
                    "nfa_dot" => {
                        let mut args = args.into_iter();
                        let set_name = Name::new(args.next().unwrap());
                        let output = format!("{}.dot", args.next().unwrap());
                        let nfa = self.get_set(&set_name).clone().to_nfa();
                        nfa.write_dot(Path::new(&output), true).unwrap();
                    }
                    "stats" => {
                        print_stats(self.get_set(&Name::new(args.into_iter().next().unwrap())))
                    }
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
