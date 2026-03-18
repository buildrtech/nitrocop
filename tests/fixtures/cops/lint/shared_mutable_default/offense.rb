Hash.new([])
^^^^^^^^^^^^ Lint/SharedMutableDefault: Do not create a Hash with a mutable default value as the default value can accidentally be changed.
Hash.new({})
^^^^^^^^^^^^ Lint/SharedMutableDefault: Do not create a Hash with a mutable default value as the default value can accidentally be changed.
Hash.new(Array.new)
^^^^^^^^^^^^^^^^^^^ Lint/SharedMutableDefault: Do not create a Hash with a mutable default value as the default value can accidentally be changed.
Hash.new(unknown: true)
^^^^^^^^^^^^^^^^^^^^^^^ Lint/SharedMutableDefault: Do not create a Hash with a mutable default value as the default value can accidentally be changed.
Hash.new(foo: 'bar', baz: 42)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/SharedMutableDefault: Do not create a Hash with a mutable default value as the default value can accidentally be changed.
Hash.new(Hash.new)
^^^^^^^^^^^^^^^^^^ Lint/SharedMutableDefault: Do not create a Hash with a mutable default value as the default value can accidentally be changed.
