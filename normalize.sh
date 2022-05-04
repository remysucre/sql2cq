for File in `ls $1`
do
	./target/release/sql2cq old/$File > temp.sql
	python fmtsql.py temp.sql > $2/$File
done

rm temp.sql
