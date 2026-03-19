# This file is auto-generated from the current state of the database.
# NOTE: t.timestamps is intentionally expanded to explicit columns because
# RuboCop's SchemaLoader crashes on t.timestamps (no arguments) when building
# Column objects — node.first_argument returns nil. This causes schema-dependent
# cops (UniqueValidationWithoutIndex, UnusedIgnoredColumns) to silently skip.
ActiveRecord::Schema[7.0].define(version: 2025_01_01_000003) do
  create_table "users", force: :cascade do |t|
    t.string "name"
    t.string "email"
    t.boolean "active"
    t.string "role"
    t.integer "department_id"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
  end

  create_table "posts", force: :cascade do |t|
    t.string "title"
    t.text "body"
    t.boolean "published"
    t.boolean "featured"
    t.integer "user_id"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
  end

  create_table "records", force: :cascade do |t|
    t.string "save"
    t.string "class"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
  end
end
