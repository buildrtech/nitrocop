# create_table without comment
create_table :users do |t|
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database table without `comment`.
  t.string :name
end

create_table :orders do |t|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database table without `comment`.
  t.integer :user_id
  t.decimal :total
end

create_table :products, force: true do |t|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database table without `comment`.
  t.string :title
end

# create_table with nil comment
create_table :items, comment: nil do |t|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database table without `comment`.
  t.string :name
end

# create_table with empty comment
create_table :categories, comment: '' do |t|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database table without `comment`.
  t.string :name
end

# add_column without comment
add_column :users, :name, :string
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.

add_column :users, :age, :integer, default: 0
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.

# add_column with nil comment
add_column :users, :role, :string, comment: nil
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.

# add_column with empty comment
add_column :users, :status, :string, comment: ''
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.

# column methods inside create_table block without comment (table HAS comment)
create_table :posts, comment: 'Posts table' do |t|
  t.string :title
  ^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.
  t.integer :views
  ^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.
  t.column :body, :text
  ^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.
end

# t.references without comment (table HAS comment)
create_table :comments, comment: 'Comments' do |t|
  t.references :user
  ^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.
end

# t.belongs_to without comment (table HAS comment)
create_table :tags, comment: 'Tags' do |t|
  t.belongs_to :post
  ^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.
end

# add_column with 2 positional args + keyword hash = 3 parser-gem args.
# RuboCop's pattern (send nil? :add_column _table _column _type _?) matches.
# The keyword hash counts as _type in the parser gem AST.
add_column :col_name, :text, null: false
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.

add_column :created_at, :timestamp, collate: '"C"'
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.

# Sequel migration — bare column calls (no receiver) inside create_table block
# RuboCop's t_column? pattern uses `_var` which matches nil receiver
create_table :guestbook_messages, comment: 'Messages people have left in the guestbook' do
  column :name, :text
  ^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.
  column :message, :text
  ^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.
  column :ip_address, :inet
  ^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/SchemaComment: New database column without `comment`.
end
