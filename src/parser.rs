use nom::{IResult};
use nom::combinator::{map_res, map, opt};
use nom::character::complete::{digit1, alpha1, multispace0};
use nom::error::VerboseError;
use crate::formula::{Atom, HiFormula};
use nom::sequence::{preceded, tuple, terminated, delimited};
use nom::branch::alt;
use nom::bytes::complete::tag;
use crate::name::Name;
use nom::multi::separated_list;
use crate::formula::{BinOp, HiPredicate};

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

fn integer(input: &str) -> IResult<&str, u64, VerboseError<&str>>
{
    map_res(digit1, |digit_str: &str| {
      digit_str.parse::<u64>()
    })(input)
}

fn variable(input: &str) -> IResult<&str, String, VerboseError<&str>>
{
    map(alpha1, |s: &str| s.to_string())(input)
}

fn atom(input: &str) -> IResult<&str, Atom, VerboseError<&str>> {
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

fn expr(input: &str) -> IResult<&str, Vec<Atom>, VerboseError<&str>> {
    separated_list(tuple((multispace0, tag("+"), multispace0)), atom)(input)
}

fn operator(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    alt((tag("=="), tag("<="), tag(">="), tag("<"), tag(">")))(input)
}

fn predicate(input: &str) -> IResult<&str, HiPredicate, VerboseError<&str>> {
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

fn formula0(input: &str) -> IResult<&str, HiFormula, VerboseError<&str>> {
    alt((delimited(tuple((tag("("), multispace0)), formula, tuple((tag(")"), multispace0))),
         map(preceded(tuple((tag("not"), multispace0)), formula0), |f| f.neg()),
         map(terminated(predicate, multispace0), HiFormula::Predicate)))(input)
}

fn formula1(input: &str) -> IResult<&str, HiFormula, VerboseError<&str>> {
    map(tuple((formula0, opt(preceded(tuple((tag("and"), multispace0)), formula1)))), |r| {
        match r {
            (f, None) => f,
            (f, Some(g)) => f.and(g),
        }
    })(input)
}

fn formula2(input: &str) -> IResult<&str, HiFormula, VerboseError<&str>> {
    map(tuple((formula1, opt(preceded(tuple((tag("or"), multispace0)), formula2)))), |r| {
        match r {
            (f, None) => f,
            (f, Some(g)) => f.or(g),
        }
    })(input)
}

/* Top level formula */
#[inline]
fn formula(input: &str) -> IResult<&str, HiFormula, VerboseError<&str>> {
    formula2(input)
}

fn varlist(input: &str) -> IResult<&str, Vec<Name>, VerboseError<&str>> {
    map(terminated(separated_list(tuple((multispace0, tag(","), multispace0)), variable), multispace0),
        |r| r.iter().map(|x| Name::from_str(x)).collect())(input)
}

fn setout(input: &str) -> IResult<&str, Vec<Name>, VerboseError<&str>> {
    terminated(varlist, tuple((multispace0, tag("|"), multispace0)))(input)
}

pub fn setdef(input: &str) -> IResult<&str, SetDef, VerboseError<&str>> {
    map(delimited(tuple((tag("{"), multispace0)),
                  tuple((setout, formula)),
                  tuple((tag("}"), multispace0))), |(vars, formula)| {
        SetDef {
            vars, formula
        }
    })(input)
}

pub fn parse_formula(input: &str) -> HiFormula {
    formula(input).unwrap().1
}

pub fn parse_setdef(input: &str) -> SetDef {
    setdef(input).unwrap().1
}

/*fn named_defset(input: &str) -> IResult<&str, SetDef, VerboseError<&str>> {
    map(tuple((variable, tuple((multispace0, tag("="), multispace0)), formula)), |(name, _, f)| {

    })(input)
}*/

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
        assert_eq!(Ok(("", f1.or(f2))), formula2("x == 4 and 2 * x == 3 * y + 3 or x == y and x == y"));

        let p1 = p("x == 4");
        let p2 = p("2 * x == 3 * y + 3");
        let p3 = p("x == y");
        let p4 = p("x == y");

        let f1 = p2.or(p3);
        assert_eq!(Ok(("", p1.and((f1.and(p4))))), formula2("x == 4 and (2 * x == 3 * y + 3 or x == y) and x == y"));
    }

    #[test]
    fn test_parser_setdef() {
        let (_, r2) = setdef("{ x, y | x <= 10 and 2 < x }").unwrap();
        let (_, f) = formula("x <= 10 and 2 < x").unwrap();
        assert_eq!(r2.vars, vec![Name::from_str("x"), Name::from_str("y")]);
        assert_eq!(r2.formula, f);
    }
}