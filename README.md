# Convert SQL to Conjunctive Queries

The current scripts do two things: 

1. "Normalizing" a `SELECT-FROM-WHERE` query, by striping away everything that is not a join predicate (`t1.col1 = t2.col2`) from the `WHERE` clause.
2. Translate the normalized query to CQ (a.k.a. tensor algebra), dropping any column that does not participate in any join.
If we kept all columns the CQ would be way too large, and we would generate extremely deep loop nests in TACO.

1. Compile with `cargo build --release` (install cargo [here](https://rustup.rs))
2. Enter pipenv with `pipenv shell` (install pipenv [here](https://pipenv.pypa.io/en/latest/))
3. Run with `cargo run 9b.sql`, where `9b.sql` is a file containing 1 SQL query.
4. Use `fmtsql.py` to format the SQL output; run `sed -E 's/(\_|\.)//g' 9b.cq` with `9b.cq` containing the CQ output to strip `.` and `_` away from the CQ.
<!-- 3. Run `bash normalize.sh old new` where `old` is a directory containing input SQL queries, one file per query, and `new` is a directory to hold the normalized queries. -->
