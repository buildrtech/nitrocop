<<~SQL
^^^^^^ Rails/SquishedSQLHeredocs: Use `<<~SQL.squish` instead of `<<~SQL`.
  SELECT * FROM posts
    WHERE id = 1
SQL

<<-SQL
^^^^^^ Rails/SquishedSQLHeredocs: Use `<<-SQL.squish` instead of `<<-SQL`.
  SELECT * FROM posts;
SQL

execute(<<~SQL, "Post Load")
        ^^^^^^ Rails/SquishedSQLHeredocs: Use `<<~SQL.squish` instead of `<<~SQL`.
  SELECT * FROM posts
    WHERE post_id = 1
SQL

# Quoted heredoc tags: <<~'SQL' and <<-'SQL'
execute <<~'SQL'
        ^^^^^^^^ Rails/SquishedSQLHeredocs: Use `<<~'SQL'.squish` instead of `<<~'SQL'`.
  SELECT * FROM records
    WHERE status = 'active'
SQL

create_function :compute, sql_definition: <<-'SQL'
                                          ^^^^^^^^^ Rails/SquishedSQLHeredocs: Use `<<-'SQL'.squish` instead of `<<-'SQL'`.
  SELECT id FROM records
SQL
