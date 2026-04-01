foo(1,
    2,
    3)
bar(:a,
    :b,
    :c)
baz("x",
    "y")

obj.set :foo => 1234,
        :bar => 'Hello World',
        :baz => 'test'

Klass[:a => :a, :b => :b,
      :c => :c,
      :d => :d]

# Misaligned &block argument (keyword hash kept as single item in with_first_argument)
comm.sudo(command,
          elevated: config.privileged,
  interactive: config.interactive,
          &handler
)

# Misaligned &block with keyword args
builder.send(:collection,
             attribute_name, collection, value_method, label_method,
             input_options, merged_input_options,
             &collection_block
)
