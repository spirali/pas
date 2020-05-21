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

use structopt::StructOpt;
use std::path::Path;
use crate::aset::AutomaticSet;
use std::fs;
use crate::parser::{setdef, parse_exact, unwrap_nom};
use crate::solver::build_set;
use crate::words::get_max;

#[derive(Debug, StructOpt)]
enum Command {
    NfaDot {
        output: String,
        #[structopt(long)]
        with_sink: bool,
        #[structopt(long)]
        reverse: bool
    },
    Range,
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
        Command::Range => {
            let names = aset.track_names().to_vec();
            let nfa = aset.to_nfa();
            for (i, name) in names.iter().enumerate() {
                println!("{:?}: {}", name, get_max(&nfa, i).to_string());
            }
        },
    };
}
