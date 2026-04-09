case foo
when *array
  bar
when 1
  baz
end
case foo
when *cond
  bar
when 4
  foobar
else
  baz
end
case foo
when *cond1
  bar
when *cond2
  doo
when 4
  foobar
else
  baz
end
case foo
when cond1, *cond2
  bar
when cond3
  baz
end
case foo
when *cond1
  bar
when 8
  barfoo
when *SOME_CONSTANT
  doo
when 4
  foobar
else
  baz
end
case foo
when *cond
  bar
when *[1, 2]
  baz
end
case foo
when Bar, *Foo
  nil
end
