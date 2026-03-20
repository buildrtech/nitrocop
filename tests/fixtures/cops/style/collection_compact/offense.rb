array.reject { |e| e.nil? }
      ^^^^^^^^^^^^^^^^^^^^^ Style/CollectionCompact: Use `compact` instead of `reject { |e| e.nil? }`.

array.reject!(&:nil?)
      ^^^^^^^^^^^^^^^ Style/CollectionCompact: Use `compact!` instead of `reject!(&:nil?)`.

hash.reject { |k, v| v.nil? }
     ^^^^^^^^^^^^^^^^^^^^^^^^ Style/CollectionCompact: Use `compact` instead of `reject { |e| e.nil? }`.

array.select { |e| !e.nil? }
      ^^^^^^^^^^^^^^^^^^^^^^ Style/CollectionCompact: Use `compact` instead of `select { |e| !e.nil? }`.

hash.select { |k, v| !v.nil? }
     ^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CollectionCompact: Use `compact` instead of `select { |e| !e.nil? }`.

hash.select! { |k, v| !v.nil? }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CollectionCompact: Use `compact!` instead of `select! { |e| !e.nil? }`.

array.filter { |e| !e.nil? }
      ^^^^^^^^^^^^^^^^^^^^^^ Style/CollectionCompact: Use `compact` instead of `filter { |e| !e.nil? }`.

hash.filter! { |k, v| !v.nil? }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CollectionCompact: Use `compact!` instead of `filter! { |e| !e.nil? }`.
