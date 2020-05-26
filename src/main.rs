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
mod render {
    pub(crate) mod png;
    pub(crate) mod dot;
}

use structopt::StructOpt;
use std::path::Path;
use crate::aset::AutomaticSet;
use std::fs;
use crate::parser::{setdef, parse_exact, unwrap_nom};
use crate::solver::build_set;
use crate::elements::{get_max_value, number_of_elements};
use std::str::FromStr;
use std::fs::File;
use std::io::BufWriter;
use crate::render::png::render_set_png;
use crate::render::dot::render_set_dot;

#[derive(Debug)]
enum RenderFormat {
    Png,
    Dot,
}

impl FromStr for RenderFormat {
    type Err = String;
    fn from_str(format: &str) -> Result<Self, Self::Err> {
        match format {
            "png" => Ok(RenderFormat::Png),
            "dot" => Ok(RenderFormat::Dot),
            _ => Err(format!("Render format '{}' does not exist", format)),
        }
    }
}

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
        format: RenderFormat,
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
        Command::Render { output, format } => {
            let dfa = aset.to_dfa();
            let file = File::create(output).unwrap();
            let mut writer = BufWriter::new(file);
            match format {
                RenderFormat::Png => render_set_png(&[&dfa], &[[255, 0, 0]], &mut writer),
                RenderFormat::Dot => render_set_dot(&dfa, &mut writer),
            }
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

            let file = File::create(output).unwrap();
            let mut writer = BufWriter::new(file);
            render_set_png(&res.iter().collect::<Vec<_>>().as_slice(), colors, &mut writer);
        }
    };
}
