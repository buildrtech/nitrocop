something
  .method_call

something.method_call

foo
  .bar
  .baz

x = object.method

# Scope resolution operator :: should not be flagged
Foo::
  bar

Foo::
  Bar
