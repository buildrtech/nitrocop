class C
  include M
end
module N
  extend O
end
Foo.include M
Class.new do
  include M
end
obj.include(M)
# Method call arguments should not be flagged (only constants)
include T('default/layout/html')
extend some_method
prepend build_module(:foo)
# include inside while/until/for/case/lambda at top level is NOT flagged by RuboCop
while condition
  include M
end
until done
  extend N
end
for x in items
  prepend O
end
case foo
when :bar
  include M
end
-> { include M }
proc { include M }
