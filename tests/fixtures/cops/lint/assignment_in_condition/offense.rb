if x = 1
     ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
  do_something
end

while y = gets
        ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
  process(y)
end

until z = calculate
        ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
  retry_something
end

if @test = 10
         ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

if @@test = 10
          ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

if $test = 10
         ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

if TEST = 10
        ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

if test == 10 || foobar = 1
                        ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

if test.method = 10
               ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

if test&.method = 10
                ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

if a[3] = 10
        ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

do_something if x = 1
                  ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.

do_something while y = gets
                     ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.

unless x = 1
         ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
  do_something
end

if (foo == bar && test = 10)
                       ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

if (foo == bar || test = 10)
                       ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

foo { x if y = z }
             ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.

raise StandardError unless (foo ||= bar) || a = b
                                              ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.

if Foo::Bar = 1
            ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

while Foo::Bar = fetch_data
               ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

unless Module::Config = load_config
                      ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
  apply_defaults
end

if Foo::Bar = 1 || baz
            ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end
