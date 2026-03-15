items.each { |x| }
^ Lint/EmptyBlock: Empty block detected.

items.each do |x|
^ Lint/EmptyBlock: Empty block detected.
end

foo { }
^ Lint/EmptyBlock: Empty block detected.

Context.create_table(:users) do |t|
  t.timestamps null: false
end.define_model do
    ^^^^^^^^^^^^ Lint/EmptyBlock: Empty block detected.
end

super(name, extensions: extensions, block: block, **kwargs) {}
^^^^^ Lint/EmptyBlock: Empty block detected.
