case x
^^^^ Style/HashLikeCase: Consider replacing `case-when` with a hash lookup.
when 'a'
  1
when 'b'
  2
when 'c'
  3
end
case y
^^^^ Style/HashLikeCase: Consider replacing `case-when` with a hash lookup.
when :foo
  'bar'
when :baz
  'qux'
when :quux
  'corge'
end

# Array literal bodies of same type (recursive_basic_literal)
case status
^^^^ Style/HashLikeCase: Consider replacing `case-when` with a hash lookup.
when :success
  ["#BackupSuccess"]
when :failure
  ["#BackupFailure"]
when :pending
  ["#BackupPending"]
end
