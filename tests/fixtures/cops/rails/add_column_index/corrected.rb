add_column :table, :column, :integer
add_index :table, :column
add_column :users, :group_id, :integer
add_index :users, :group_id
add_column :posts, :category_id, :bigint, null: false
add_index :posts, :category_id, unique: true
