# Convert SQL to Conjunctive Queries

The current scripts "normalizes" a `SELECT-FROM-WHERE` query, by striping away everything that is not a join predicate (`t1.col1 = t2.col2`) from the `WHERE` clause.

1. Compile with `cargo build --release`
2. Enter pipenv with pipenv shell
3. Run `bash normalize.sh old new` where old is a directory containing input SQL queries, one file per query, and new is a directory to hold the normalized queries.
