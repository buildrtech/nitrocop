x = {
  a: 1,
  b: 2,
  c: 3
}

y = {
  foo: :bar,
  baz: :qux,
  quux: :corge
}

z = {
  one: 1,
  two: 2,
  three: 3
}

# Offending multiline element: :settings shares line with :app, but :defaults
# on the END line of :settings should NOT be flagged (last_seen_line algorithm).
w = {:app => {},
     :settings => {:logger => ["/tmp/2.log"],
  :logger_level => 2}, :defaults => {}}

# Multiple elements share a line, one has a multiline value; the element after
# the multiline value's end line is NOT an offense.
v = {'id' => records.first.id,
     'label' => 'updated',
                               'action' =>
  {'type' => 'Field', 'attrs' => {}}, 'responders' => []}
