top.test

foo.bar

obj&.baz

# it() with receiver is flagged
0.times { foo.it }

# it() in def body is flagged
def foo
  it
end

# it() in block with explicit empty params is flagged
0.times { ||
  it
}

# it() in block with named params is flagged
0.times { |_n|
  it
}

# Same-name assignment with receiver is still flagged
test = x.test

# obj.method ||= func() — the func() is flagged
obj.method ||= func

# obj.method += func() — the func() is flagged
obj.method += func

# Mass assignment where LHS is a send (c[2]) — method with same name is flagged
c[2], x = c
