from ast import keyword
import sqlparse
import argparse

parser = argparse.ArgumentParser(description='Format a SQL query.')
parser.add_argument('file', metavar='F', type=str, 
                    help='a file containing a SQL statement')

args = parser.parse_args()
file_name = args.file

with open(file_name) as f:
    sql = f.read()
    statements = sqlparse.split(sql)
    print(sqlparse.format(statements[0], reindent=True))
