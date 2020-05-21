use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{alpha1, char, digit1, multispace0};
use nom::combinator::{all_consuming, map, map_res, opt};
use nom::error::{convert_error, VerboseError};
use nom::IResult;
use nom::multi::{fold_many0, separated_list};
use nom::sequence::{delimited, pair, preceded, terminated, tuple};

use crate::formula::{Atom, HiFormula};
use crate::formula::{BinOp, HiPredicate};
use crate::name::Name;

pub type NomResult<'a, Ret> = IResult<&'a str, Ret, VerboseError<&'a str>>;

pub struct SetDef {
    vars: Vec<Name>,
    formula: HiFormula,
}

impl SetDef {
    pub fn vars(&self) -> &[Name] {
        &self.vars
    }

    pub fn formula(&self) -> &HiFormula {
        &self.formula
    }
}

fn integer(input: &str) -> NomResult<u64>
{
    map_res(digit1, |digit_str: &str| {
        digit_str.parse::<u64>()
    })(input)
}

fn variable(input: &str) -> NomResult<String>
{
    map(alpha1, |s: &str| s.to_string())(input)
}

fn atom(input: &str) -> NomResult<Atom> {
    alt((
        map(tuple((integer, opt(preceded(tuple((multispace0, tag("*"), multispace0)), variable)))), |r| {
            match r {
                (value, None) => Atom::Constant(value),
                (value, Some(name)) => Atom::Variable(Name::new(name), value)
            }
        }),
        map(variable, |r| Atom::Variable(Name::new(r), 1)),
    ))(input)
}

fn expr(input: &str) -> NomResult<Vec<Atom>> {
    separated_list(tuple((multispace0, tag("+"), multispace0)), atom)(input)
}

fn operator(input: &str) -> NomResult<&str> {
    alt((tag("=="), tag("<="), tag(">="), tag("<"), tag(">")))(input)
}

fn predicate(input: &str) -> NomResult<HiPredicate> {
    map(tuple((expr, delimited(multispace0, operator, multispace0), expr)), |(lhs, op, rhs)| {
        match op {
            "==" => HiPredicate::BinOp(BinOp::Eq, lhs, rhs),
            "<=" => HiPredicate::BinOp(BinOp::Lte, lhs, rhs),
            "<" => HiPredicate::BinOp(BinOp::Lt, lhs, rhs),
            ">=" => HiPredicate::BinOp(BinOp::Lte, rhs, lhs),
            ">" => HiPredicate::BinOp(BinOp::Lt, rhs, lhs),
            _ => unreachable!()
        }
    })(input)
}

/*pub fn and_or(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    map(tuple((multispace0, opt((tag("and"), tag("or"))), multispace0)), |(_, r, _)| r)
}*/

#[derive(Clone, Debug)]
enum Quantifier {
    Exists(Name),
    ForAll(Name),
}

fn quantifier(input: &str) -> NomResult<Quantifier> {
    map(pair(
        terminated(alt((tag("exists"), tag("forall"))), multispace0),
        delimited(
            pair(tag("("), multispace0),
            variable,
            pair(tag(")"), multispace0),
        ),
    ), |r| {
        let name = Name::Named(r.1);
        match r.0 {
            "exists" => Quantifier::Exists(name),
            "forall" => Quantifier::ForAll(name),
            _ => unreachable!()
        }
    })(input)
}

fn quantifiers(input: &str) -> NomResult<Vec<Quantifier>> {
    fold_many0(quantifier, Vec::new(), |mut acc: Vec<_>, item| {
        acc.push(item);
        acc
    })(input)
}

fn formula_inner(input: &str) -> NomResult<HiFormula> {
    alt((
        map(tuple((quantifiers, delimited(pair(tag("("), multispace0), formula, pair(tag(")"), multispace0)))), |r| {
            r.0.into_iter().rev().fold(r.1, |acc, item| match item {
                Quantifier::Exists(name) => HiFormula::Exists(name, Box::new(acc)),
                Quantifier::ForAll(name) => HiFormula::ForAll(name, Box::new(acc))
            })
        }),
        map(preceded(tuple((tag("not"), multispace0)), formula_inner), |f| f.neg()),
        map(terminated(predicate, multispace0), HiFormula::Predicate)
    ))(input)
}

fn formula_and(input: &str) -> NomResult<HiFormula> {
    map(tuple((formula_inner, opt(preceded(tuple((tag("and"), multispace0)), formula_and)))), |r| {
        match r {
            (f, None) => f,
            (f, Some(g)) => f.and(g),
        }
    })(input)
}

fn formula_or(input: &str) -> NomResult<HiFormula> {
    map(tuple((formula_and, opt(preceded(tuple((tag("or"), multispace0)), formula_or)))), |r| {
        match r {
            (f, None) => f,
            (f, Some(g)) => f.or(g),
        }
    })(input)
}

/* Top level formula */
#[inline]
fn formula(input: &str) -> IResult<&str, HiFormula, VerboseError<&str>> {
    formula_or(input)
}

fn varlist(input: &str) -> NomResult<Vec<Name>> {
    map(terminated(separated_list(tuple((multispace0, tag(","), multispace0)), variable), multispace0),
        |r| r.iter().map(|x| Name::from_str(x)).collect())(input)
}

fn setout(input: &str) -> NomResult<Vec<Name>> {
    terminated(varlist, tuple((multispace0, tag("|"), multispace0)))(input)
}

pub fn setdef(input: &str) -> NomResult<SetDef> {
    map(delimited(tuple((tag("{"), multispace0)),
                  tuple((setout, formula)),
                  tuple((tag("}"), multispace0))), |(vars, formula)| {
        SetDef {
            vars,
            formula,
        }
    })(input)
}

pub fn parse_formula(input: &str) -> HiFormula {
    formula(input).unwrap().1
}

pub fn parse_setdef(input: &str) -> SetDef {
    setdef(input).unwrap().1
}

/*fn named_defset(input: &str) -> NomResult<SetDef> {
    map(tuple((variable, tuple((multispace0, tag("="), multispace0)), formula)), |(name, _, f)| {

    })(input)
}*/

pub fn parse_exact<Ret, Parser: Fn(&str) -> NomResult<Ret>>(parser: Parser, input: &str) -> NomResult<Ret> {
    all_consuming(parser)(input)
}

pub fn unwrap_nom<'a, Ret>(input: &'a str, result: NomResult<'a, Ret>) -> (&'a str, Ret) {
    match result {
        Ok(data) => data,
        Err(e) => match e {
            nom::Err::Incomplete(needed) => panic!("Incomplete input"),
            nom::Err::Error(e) | nom::Err::Failure(e) => panic!(convert_error(input, e))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_integer() {
        assert_eq!(Ok(("", 0)), integer("0"));
        assert_eq!(Ok(("", 123400)), integer("123400"));
        assert_eq!(Ok(("", 00123400)), integer("123400"));
    }

    #[test]
    fn parse_atom() {
        assert_eq!(Ok(("", Atom::Variable(Name::from_str("hello"), 1))), atom("hello"));
        assert_eq!(Ok(("", Atom::Variable(Name::from_str("hello"), 23))), atom("23 * hello"));
        assert_eq!(Ok(("", Atom::Variable(Name::from_str("hello"), 0))), atom("0*hello"));
        assert_eq!(Ok(("", Atom::Constant(17))), atom("17"));
    }

    #[test]
    fn parse_expr() {
        let x1 = Atom::Variable(Name::from_str("x"), 1);
        assert_eq!(Ok(("", vec![x1, Atom::Constant(2)])), expr("x + 2"));
        assert_eq!(Ok(("", vec![Atom::Constant(17)])), expr("17"));
        let xx2 = Atom::Variable(Name::from_str("xx"), 2);
        let yy2 = Atom::Variable(Name::from_str("yy"), 3);
        assert_eq!(Ok(("", vec![xx2, yy2])), expr("2 * xx + 3 * yy"));
    }

    #[test]
    fn test_parse_predicate() {
        let x1 = Atom::Variable(Name::from_str("x"), 1);
        let y3 = Atom::Variable(Name::from_str("y"), 3);
        let c2 = Atom::Constant(2);
        assert_eq!(Ok(("", HiPredicate::BinOp(BinOp::Eq, vec![x1, c2], vec![y3]))), predicate("x + 2 == 3 * y"));
    }

    #[test]
    fn test_parse_formula() {
        let p = |s: &str| {
            HiFormula::Predicate(predicate(s).unwrap().1)
        };

        let p1 = p("x == 4");
        let p2 = p("2 * x == 3 * y + 3");
        let p3 = p("x == y");
        let p4 = p("x == y");

        let f1 = p1.and(p2);
        let f2 = p3.and(p4);
        assert_eq!(Ok(("", f1.or(f2))), formula_or("x == 4 and 2 * x == 3 * y + 3 or x == y and x == y"));

        let p1 = p("x == 4");
        let p2 = p("2 * x == 3 * y + 3");
        let p3 = p("x == y");
        let p4 = p("x == y");

        let f1 = p2.or(p3);
        assert_eq!(Ok(("", p1.and((f1.and(p4))))), formula_or("x == 4 and (2 * x == 3 * y + 3 or x == y) and x == y"));
    }

    #[test]
    fn test_parser_setdef() {
        let (_, r2) = setdef("{ x, y | x <= 10 and 2 < x }").unwrap();
        let (_, f) = formula("x <= 10 and 2 < x").unwrap();
        assert_eq!(r2.vars, vec![Name::from_str("x"), Name::from_str("y")]);
        assert_eq!(r2.formula, f);
    }

    #[test]
    fn test_parser_exact() {
        assert!(parse_exact(setdef, "{ x, y | x <= 10 and 2 < x } + 1").is_err());
    }

    #[test]
    fn test_parser_quantifiers() {
        let (_, f) = formula("forall(x) exists(y) (x < y)").unwrap();
        let x = String::from("x");
        let y = String::from("y");
        let exists = Box::new(HiFormula::Exists(Name::Named(y.clone()), Box::new(HiFormula::Predicate(
            HiPredicate::BinOp(BinOp::Lt,
                               vec!(Atom::from_name(Name::new(x.clone()))),
                               vec!(Atom::from_name(Name::new(y))),
            ))
        )));
        assert_eq!(f, HiFormula::ForAll(Name::Named(x), exists));
    }
}
