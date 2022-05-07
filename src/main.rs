#[macro_use]
extern crate lazy_static;

use indexmap::IndexMap;
use sqlparser::ast::*;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use std::collections::HashMap;
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

            let mut filters = vec![];
            let mut joins = vec![];

            if let Some(sel) = &q.selection {
                get_joins(sel, &mut joins, &mut filters);
            }

            let mut uf = UnionFind::default();
            let mut col_id = IndexMap::new();

            for (l, r) in joins.iter() {
                let next_id = col_id.len();
                let l_id = *col_id.entry(l).or_insert(next_id);
                if uf.size() < col_id.len() {
                    uf.make_set();
                }
                assert_eq!(uf.size(), col_id.len());

                let next_id = col_id.len();
                let r_id = *col_id.entry(r).or_insert(next_id);
                if uf.size() < col_id.len() {
                    uf.make_set();
                }
                assert_eq!(uf.size(), col_id.len());

                let ll = uf.find_mut(l_id);
                let lr = uf.find_mut(r_id);
                uf.union(ll, lr);
            }

            let frm = &q.from;

            let mut atoms = vec![];

            for t in frm {
                let r = &t.relation;
                if let TableFactor::Table { name, alias, .. } = r {
                    let cols = SCHEMA.get(&name.0[0].value).unwrap();
                    let rel = &alias.as_ref().unwrap().name.value;
                    let mut vars = vec![];
                    for col in cols {
                        let idents = vec![
                            alias.as_ref().unwrap().name.clone(),
                            Ident { value: col.clone(), quote_style: None }, 
                            ];

                        if let Some(id) = col_id.get(&idents) {
                            let col_str = *col_id.get_index(uf.find(*id)).unwrap().0;
                            let mut var = vec![];
                            for s in col_str {
                                var.push(s.value.clone());
                            }
                            vars.push(var.join("."));
                        }
                    }
                    atoms.push(format!("{}({})", rel, vars.join(",")));
                } else {
                    panic!("Not supported");
                }
            }

            // println!("q :- {}.", atoms.join(", "));
            println!("q = {}", atoms.join(" * "));

            let j = joins.pop().expect("Query has no joins");
            q.selection = Some(joins.drain(..).fold(mk_join(j), |l, r| Expr::BinaryOp {
                left: Box::new(l),
                op: BinaryOperator::And,
                right: Box::new(mk_join(r)),
            }));
        } else {
            panic!("Only SELECT-PROJECT-JOIN queries are supported");
        }
    } else {
        panic!("Only SELECT queries are supported");
    }

    println!("{};", stmt);
}

fn mk_join(lr: (Vec<Ident>, Vec<Ident>)) -> Expr {
    let (l, r) = lr;
    Expr::BinaryOp { 
        left: Box::new(Expr::CompoundIdentifier(l)), 
        op: BinaryOperator::Eq, 
        right: Box::new(Expr::CompoundIdentifier(r)),
    }
}

fn get_joins(e: &Expr, joins: &mut Vec<(Vec<Ident>, Vec<Ident>)>, filters: &mut Vec<Expr>) {
    if let Expr::BinaryOp {
        left: l,
        op: o,
        right: r,
    } = e
    {
        match (&**l, o, &**r) {
            (Expr::CompoundIdentifier(l), BinaryOperator::Eq, Expr::CompoundIdentifier(r)) => {
                joins.push((l.clone(), r.clone()))
            }
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

type Id = usize;

#[derive(Debug, Clone, Default)]
pub struct UnionFind {
    parents: Vec<Id>,
}

impl UnionFind {
    pub fn make_set(&mut self) -> Id {
        let id = Id::from(self.parents.len());
        self.parents.push(id);
        id
    }

    pub fn size(&self) -> usize {
        self.parents.len()
    }

    fn parent(&self, query: Id) -> Id {
        self.parents[usize::from(query)]
    }

    fn parent_mut(&mut self, query: Id) -> &mut Id {
        &mut self.parents[usize::from(query)]
    }

    pub fn find(&self, mut current: Id) -> Id {
        while current != self.parent(current) {
            current = self.parent(current)
        }
        current
    }

    pub fn find_mut(&mut self, mut current: Id) -> Id {
        while current != self.parent(current) {
            let grandparent = self.parent(self.parent(current));
            *self.parent_mut(current) = grandparent;
            current = grandparent;
        }
        current
    }

    /// Given two leader ids, unions the two eclasses making root1 the leader.
    pub fn union(&mut self, root1: Id, root2: Id) -> Id {
        *self.parent_mut(root2) = root1;
        root1
    }
}

lazy_static!{
    static ref SCHEMA: HashMap<String, Vec<String>> = {
        let dialect = GenericDialect {};
        let schema_stmts = Parser::parse_sql(&dialect, SCHEMA_STR).unwrap();
        let mut schema = HashMap::new();
        for stmt in schema_stmts {
            if let Statement::CreateTable { 
                or_replace: _, temporary: _, external: _, if_not_exists: _, 
                name, columns, ..} = stmt 
                {
                    let cols: Vec<_> = columns.iter().map(|col| col.name.value.clone()).collect();
                    schema.insert(name.0[0].value.clone(), cols);
                } else {
                    panic!("SCHEMA_STR contains a statement that is not CREATE TABLE");
                }
        };
        schema
    };
}

static SCHEMA_STR: &str = "
CREATE TABLE aka_name (
    id integer NOT NULL PRIMARY KEY,
    person_id integer NOT NULL,
    name text NOT NULL,
    imdb_index character varying(12),
    name_pcode_cf character varying(5),
    name_pcode_nf character varying(5),
    surname_pcode character varying(5),
    md5sum character varying(32)
);

CREATE TABLE aka_title (
    id integer NOT NULL PRIMARY KEY,
    movie_id integer NOT NULL,
    title text NOT NULL,
    imdb_index character varying(12),
    kind_id integer NOT NULL,
    production_year integer,
    phonetic_code character varying(5),
    episode_of_id integer,
    season_nr integer,
    episode_nr integer,
    note text,
    md5sum character varying(32)
);

CREATE TABLE cast_info (
    id integer NOT NULL PRIMARY KEY,
    person_id integer NOT NULL,
    movie_id integer NOT NULL,
    person_role_id integer,
    note text,
    nr_order integer,
    role_id integer NOT NULL
);

CREATE TABLE char_name (
    id integer NOT NULL PRIMARY KEY,
    name text NOT NULL,
    imdb_index character varying(12),
    imdb_id integer,
    name_pcode_nf character varying(5),
    surname_pcode character varying(5),
    md5sum character varying(32)
);

CREATE TABLE comp_cast_type (
    id integer NOT NULL PRIMARY KEY,
    kind character varying(32) NOT NULL
);

CREATE TABLE company_name (
    id integer NOT NULL PRIMARY KEY,
    name text NOT NULL,
    country_code character varying(255),
    imdb_id integer,
    name_pcode_nf character varying(5),
    name_pcode_sf character varying(5),
    md5sum character varying(32)
);

CREATE TABLE company_type (
    id integer NOT NULL PRIMARY KEY,
    kind character varying(32) NOT NULL
);

CREATE TABLE complete_cast (
    id integer NOT NULL PRIMARY KEY,
    movie_id integer,
    subject_id integer NOT NULL,
    status_id integer NOT NULL
);

CREATE TABLE info_type (
    id integer NOT NULL PRIMARY KEY,
    info character varying(32) NOT NULL
);

CREATE TABLE keyword (
    id integer NOT NULL PRIMARY KEY,
    keyword text NOT NULL,
    phonetic_code character varying(5)
);

CREATE TABLE kind_type (
    id integer NOT NULL PRIMARY KEY,
    kind character varying(15) NOT NULL
);

CREATE TABLE link_type (
    id integer NOT NULL PRIMARY KEY,
    link character varying(32) NOT NULL
);

CREATE TABLE movie_companies (
    id integer NOT NULL PRIMARY KEY,
    movie_id integer NOT NULL,
    company_id integer NOT NULL,
    company_type_id integer NOT NULL,
    note text
);

CREATE TABLE movie_info (
    id integer NOT NULL PRIMARY KEY,
    movie_id integer NOT NULL,
    info_type_id integer NOT NULL,
    info text NOT NULL,
    note text
);

CREATE TABLE movie_info_idx (
    id integer NOT NULL PRIMARY KEY,
    movie_id integer NOT NULL,
    info_type_id integer NOT NULL,
    info text NOT NULL,
    note text
);

CREATE TABLE movie_keyword (
    id integer NOT NULL PRIMARY KEY,
    movie_id integer NOT NULL,
    keyword_id integer NOT NULL
);

CREATE TABLE movie_link (
    id integer NOT NULL PRIMARY KEY,
    movie_id integer NOT NULL,
    linked_movie_id integer NOT NULL,
    link_type_id integer NOT NULL
);

CREATE TABLE name (
    id integer NOT NULL PRIMARY KEY,
    name text NOT NULL,
    imdb_index character varying(12),
    imdb_id integer,
    gender character varying(1),
    name_pcode_cf character varying(5),
    name_pcode_nf character varying(5),
    surname_pcode character varying(5),
    md5sum character varying(32)
);

CREATE TABLE person_info (
    id integer NOT NULL PRIMARY KEY,
    person_id integer NOT NULL,
    info_type_id integer NOT NULL,
    info text NOT NULL,
    note text
);

CREATE TABLE role_type (
    id integer NOT NULL PRIMARY KEY,
    role character varying(32) NOT NULL
);

CREATE TABLE title (
    id integer NOT NULL PRIMARY KEY,
    title text NOT NULL,
    imdb_index character varying(12),
    kind_id integer NOT NULL,
    production_year integer,
    imdb_id integer,
    phonetic_code character varying(5),
    episode_of_id integer,
    season_nr integer,
    episode_nr integer,
    series_years character varying(49),
    md5sum character varying(32)
);
";
