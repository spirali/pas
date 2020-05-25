mod common;
mod name;
mod table;
mod dfa;
mod nfa;
mod automaton;
mod aset;
mod formula;
mod solver;
mod parser;
mod words;
mod elements;
mod render;


use structopt::StructOpt;
use std::path::Path;
use crate::aset::AutomaticSet;
use std::fs;
use crate::parser::{setdef, parse_exact, unwrap_nom};
use crate::solver::build_set;
use crate::elements::{get_max_value, number_of_elements};
use crate::render::render_set;

#[derive(Debug, StructOpt)]
enum Command {
    NfaDot {
        output: String,
        #[structopt(long)]
        with_sink: bool,
        #[structopt(long)]
        reverse: bool
    },
    Render {
        output: String,
    },
    Split {
        output: String,
    },
    Stats,
    IsEmpty
}

#[derive(Debug, StructOpt)]
struct Opts {
    #[structopt(long)]
    file: String,
    #[structopt(subcommand)]
    command: Command,
}

fn read_file(path: &Path) -> AutomaticSet {
    let content = fs::read_to_string(path).unwrap();
    let input = content.trim();
    let (_, sdef) = unwrap_nom(input, parse_exact(setdef, input));
    build_set(&sdef)
}

fn print_stats(aset: AutomaticSet) {
    let names = aset.track_names().to_vec();
    let dfa = aset.to_dfa();
    println!("DFA size: {}", dfa.n_states());
    let nfa = dfa.to_nfa();
    for (i, name) in names.iter().enumerate() {
        println!("Max {:?}: {}", name, get_max_value(&nfa, i).to_string());
    }
}

fn main() {
    let opts = Opts::from_args();
    let aset = read_file(Path::new(&opts.file));
    match opts.command {
        Command::NfaDot { output, with_sink, reverse } => {
            let nfa = if reverse {
                aset.to_dfa().reverse().determinize().minimize().to_nfa()
            } else {
                aset.to_nfa()
            };
            nfa.write_dot(Path::new(&output), !with_sink).unwrap()
        },
        Command::Stats => {
            print_stats(aset);
        },
        Command::IsEmpty => {
            println!("Empty: {}", aset.is_empty());
        }
        Command::Render { output } => {
            let dfa = aset.to_dfa();
            render_set(&[&dfa], &[[255, 0, 0]], Path::new(&output));
        }
        Command::Split { output } => {
            //let colors = &[[255, 0, 0], [0, 255, 0], [0, 0, 255], [0, 128, 128], [128, 128, 0]];
            let colors = &[[255, 0, 0], [0, 255, 0], [0, 0, 255]];
            let count = colors.len();
            let size = aset.size().unwrap() / count;
            println!("TARGET SIZE {} {}", size, aset.size().unwrap());
            let mut aset = aset;

            let mut res = Vec::new();
            for _ in 0..count - 1 {
                let (a, b) = aset.cut2(size);
                println!("ASIZE {:?} {:?}", a.size(), b.size());
                res.push(a.to_dfa());
                aset = b;
            }
            res.push(aset.to_dfa());

            for r in &res {
                println!("SIZE: {}", number_of_elements(r).unwrap());
            }

            render_set(&res.iter().collect::<Vec<_>>().as_slice(), colors, Path::new(&output));
        }
    };
}
