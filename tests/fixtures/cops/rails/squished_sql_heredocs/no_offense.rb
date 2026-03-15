<<-SQL.squish
  SELECT * FROM posts
    WHERE id = 1
SQL

<<~SQL.squish
  SELECT * FROM posts
    WHERE id = 1
SQL

<<-SQL
  SELECT * FROM posts
  -- This is a comment, so squish can't be used
    WHERE id = 1
SQL

execute(<<~SQL.squish, "Post Load")
  SELECT * FROM posts
    WHERE post_id = 1
SQL

<<~SQL
  SELECT * FROM posts
  WHERE id = 1 -- filter active records
SQL

<<-SQL
  SELECT name FROM users
  WHERE role = 'admin' -- only admins
  ORDER BY name
SQL

<<~SQL
  SELECT "col--name" FROM posts
  WHERE status = 1 -- inline comment
SQL

# Quoted tags with squish are fine
<<~'SQL'.squish
  SELECT * FROM records
SQL

<<-'SQL'.squish
  SELECT id FROM items
SQL
