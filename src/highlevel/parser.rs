use nom::{InputTakeAtPosition, IResult};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{alpha1, char, digit1, multispace0};
use nom::combinator::{all_consuming, map, map_res, opt};
use nom::error::{convert_error, ErrorKind, VerboseError};
use nom::multi::{fold_many0, separated_list};
use nom::sequence::{delimited, pair, preceded, terminated, tuple};

use crate::common::Name;
use crate::highlevel::hiformula::{Expression, HiFormula};
use crate::highlevel::hiformula::{BinOp, HiPredicate};
use crate::solver::commands::{Command, SetDef};

pub type NomResult<'a, Ret> = IResult<&'a str, Ret, VerboseError<&'a str>>;

fn integer(input: &str) -> NomResult<u64>
{
    map_res(digit1, |digit_str: &str| {
        digit_str.parse::<u64>()
    })(input)
}

pub fn is_id_char(c: char) -> bool {
    match c {
        'A'..='Z' | 'a'..='z' | '_' => true,
        _ => false,
    }
}

fn identifier(input: &str) -> NomResult<String>
{
    input.split_at_position1_complete(|item| !is_id_char(item), ErrorKind::Alpha).map(|(x, y)| (x, y.to_string()))
}

fn atom(input: &str) -> NomResult<Expression> {
    alt((
        map(tuple((integer, opt(preceded(tuple((multispace0, tag("*"), multispace0)), identifier)))), |r| {
            match r {
                (value, None) => Expression::Constant(value),
                (value, Some(name)) => Expression::Mul(Box::new(Expression::Variable(Name::new(name))), value),
            }
        }),
        map(tuple((identifier, opt(preceded(tuple((multispace0, tag("%"), multispace0)), integer)))), |r| {
            match r {
                (name, None) => Expression::Variable(Name::new(name)),
                (name, Some(value)) => Expression::Mod(Box::new(Expression::Variable(Name::new(name))), value)
            }
        })
    ))(input)
}

fn expr(input: &str) -> NomResult<Vec<Expression>> {
    separated_list(tuple((multispace0, tag("+"), multispace0)), atom)(input)
}

fn operator(input: &str) -> NomResult<&str> {
    alt((tag("=="), tag("<="), tag(">="), tag("<"), tag(">")))(input)
}

fn predicate(input: &str) -> NomResult<HiPredicate> {
    map(tuple((expr, delimited(multispace0, operator, multispace0), expr)), |(lhs, op, rhs)| {
        let lhs = Expression::new_add(lhs);
        let rhs = Expression::new_add(rhs);
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
            identifier,
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

fn id_list(input: &str) -> NomResult<Vec<String>> {
    terminated(separated_list(tuple((multispace0, tag(","), multispace0)), identifier), multispace0)(input)
}

fn var_list(input: &str) -> NomResult<Vec<Name>> {
    map(id_list, |r| r.iter().map(|x| Name::from_str(x)).collect())(input)
}

fn setout(input: &str) -> NomResult<Vec<Name>> {
    terminated(var_list, tuple((multispace0, tag("|"), multispace0)))(input)
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

pub fn command(input: &str) -> NomResult<Command> {
    alt((
        map(tuple((identifier, delimited(multispace0, tag("="), multispace0), setdef)), |(name, _, sd)| Command::SetDef(name, sd)),
        map(tuple((identifier, delimited(delimited(multispace0, tag("("), multispace0), id_list, delimited(multispace0, tag(")"), multispace0)))), |(name, args)| Command::Call(name, args)),
    ))(input)
}

pub fn commands(input: &str) -> NomResult<Vec<Command>> {
    terminated(separated_list(tuple((multispace0, tag(";"), multispace0)), command), multispace0)(input)
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
        assert_eq!(Ok(("", Expression::Variable(Name::from_str("hello")))), atom("hello"));
        assert_eq!(Ok(("", Expression::Mul(Box::new(Expression::Variable(Name::from_str("hello"))), 23))), atom("23 * hello"));
        assert_eq!(Ok(("", Expression::Mul(Box::new(Expression::Variable(Name::from_str("hello"))), 0))), atom("0*hello"));
        assert_eq!(Ok(("", Expression::Constant(17))), atom("17"));
        assert_eq!(Ok(("", Expression::Mod(Box::new(Expression::Variable(Name::from_str("hello"))), 23))), atom("hello % 23"));
    }

    #[test]
    fn parse_expr() {
        let x1 = Expression::Variable(Name::from_str("x"));
        assert_eq!(Ok(("", vec![x1, Expression::Constant(2)])), expr("x + 2"));
        assert_eq!(Ok(("", vec![Expression::Constant(17)])), expr("17"));
        let xx2 = Expression::Variable(Name::from_str("xx"));
        let yy2 = Expression::Variable(Name::from_str("yy"));
        assert_eq!(Ok(("", vec![Expression::Mul(Box::new(xx2), 2), Expression::Mul(Box::new(yy2), 3)])), expr("2 * xx + 3 * yy"));
    }

    #[test]
    fn test_parse_predicate() {
        let x1 = Expression::Variable(Name::from_str("x"));
        let y3 = Expression::Mul(Box::new(Expression::Variable(Name::from_str("y"))), 3);
        let c2 = Expression::Constant(2);
        assert_eq!(Ok(("", HiPredicate::BinOp(BinOp::Eq, Expression::Add(vec![x1, c2]), y3))), predicate("x + 2 == 3 * y"));
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
    fn test_parser_commands() {
        let (_, cs) = parse_exact(commands, "aa = { x | x == x }").unwrap();
        assert_eq!(cs.len(), 1);
        let (_, cs) = parse_exact(commands, "set = { x | x == y + 1}; print(set)").unwrap();
        assert_eq!(cs.len(), 2);
    }

    #[test]
    fn test_parser_quantifiers() {
        let (_, f) = formula("forall(x) exists(y) (x < y)").unwrap();
        let x = String::from("x");
        let y = String::from("y");
        let exists = Box::new(HiFormula::Exists(Name::Named(y.clone()), Box::new(HiFormula::Predicate(
            HiPredicate::BinOp(BinOp::Lt,
                               Expression::from_name(Name::new(x.clone())),
                               Expression::from_name(Name::new(y))),
        )),
        ));
        assert_eq!(f, HiFormula::ForAll(Name::Named(x), exists));
    }
}
