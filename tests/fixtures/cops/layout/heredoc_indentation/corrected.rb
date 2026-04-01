x = <<~RUBY
something
RUBY

y = <<~TEXT
hello world
TEXT

z = <<~SQL
SELECT * FROM users
SQL

# <<- with .squish and indented body should be flagged
execute <<~SQL.squish
  INSERT INTO accounts (name) VALUES ('test')
SQL

result = ActiveRecord::Base.connection.exec_insert(<<~SQL.squish)
    SELECT id, name
    FROM users
    WHERE id = 1
SQL

Status.find_by_sql(<<~SQL.squish)
      WITH RECURSIVE search_tree(id, path) AS (
        SELECT id, ARRAY[id] FROM statuses WHERE id = :id
      )
      SELECT id FROM search_tree
SQL

# <<- with .squish on separate line after closing delimiter
query = <<~SQL
  SELECT MAX(title)
    FROM articles
SQL
.squish

join = <<~SQL
  LEFT OUTER JOIN (
      SELECT comments.*
    FROM comments
  ) AS latest_comment
SQL
.squish

# <<- with .squish! on separate line after closing delimiter
value = <<~TEXT
  some content here
TEXT
.squish!

# Bare <<WORD heredocs with body at column 0 should be flagged
a = <<~RUBY
something
RUBY

b = <<~TEXT
hello world
TEXT

c = <<~SQL
SELECT * FROM users
SQL
