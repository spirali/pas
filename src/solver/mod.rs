pub use aset::AutomaticSet;
pub use elements::{cut, Element, get_max_value, get_nth_element, iterate_elements, number_of_elements};
pub use eval::{evaluate_formula, evaluate_predicate};
pub use formula::{LoFormula, LoPredicate};

pub mod aset;
pub mod elements;
pub mod formula;
pub mod eval;
pub mod commands;

