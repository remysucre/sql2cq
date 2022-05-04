use sqlparser::ast::*;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let f = &args[1];

    let sql = fs::read_to_string(f).expect("Unable to read file");

    let dialect = GenericDialect {};

    let mut ast = Parser::parse_sql(&dialect, &sql).unwrap();
    assert_eq!(ast.len(), 1, "File must contain exactly 1 statement");

    let mut stmt = ast.pop().unwrap();

    if let Statement::Query(q) = &mut stmt {
        if let SetExpr::Select(q) = &mut q.body {
            // TODO process filters
            let mut filters = vec![];
            let mut joins = vec![];

            if let Some(sel) = &q.selection {
                get_joins(sel, &mut joins, &mut filters);
            }

            let j = joins.pop().expect("Query has no joins");
            q.selection = Some(joins.drain(..).fold(j, |l, r| Expr::BinaryOp {
                left: Box::new(l),
                op: BinaryOperator::And,
                right: Box::new(r),
            }));
        } else {
            panic!("Only SELECT-PROJECT-JOIN queries are supported");
        }
    } else {
        panic!("Only SELECT queries are supported");
    }

    println!("{};", stmt);
}

fn get_joins(e: &Expr, joins: &mut Vec<Expr>, filters: &mut Vec<Expr>) {
    if let Expr::BinaryOp {
        left: l,
        op: o,
        right: r,
    } = e
    {
        match (&**l, o, &**r) {
            (Expr::CompoundIdentifier(_), BinaryOperator::Eq, Expr::CompoundIdentifier(_)) => {
                joins.push(e.clone())
            }
            // TODO
            (e_l, BinaryOperator::And, e_r) => {
                get_joins(e_l, joins, filters);
                get_joins(e_r, joins, filters)
            }
            _ => filters.push(e.clone()),
        }
    } else {
        filters.push(e.clone());
    }
}
