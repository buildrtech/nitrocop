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
