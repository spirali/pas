use std::fs;
use std::path::Path;
use std::str::FromStr;

use structopt::StructOpt;

use crate::highlevel::parser::{commands, parse_exact, setdef, unwrap_nom};
use crate::render::dot::render_set_dot;
use crate::render::png::render_set_png;
use crate::solver::aset::AutomaticSet;
use crate::solver::commands::{Command, Context};
use crate::solver::elements::{get_max_value, iterate_elements, number_of_elements};

pub mod common;
pub mod solver;
pub mod automata;
pub mod highlevel;
pub mod render;


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
struct Opts {
    file: String,
}

fn read_file(path: &Path) -> Vec<Command> {
    let content = fs::read_to_string(path).unwrap();
    let input = content.trim();
    let (_, cmds) = unwrap_nom(input, parse_exact(commands, input));
    cmds
}

fn main() {
    let opts = Opts::from_args();
    let cmds = read_file(Path::new(&opts.file));
    let mut context = Context::new();

    for cmd in cmds {
        context.eval(cmd);
    }
}
