one_plus_one = 1\
                ^ Layout/LineContinuationSpacing: Use one space before backslash.
  + 1

two_plus_two = 2   \
                ^^^^ Layout/LineContinuationSpacing: Use one space before backslash.
  + 2

three = 3\
         ^ Layout/LineContinuationSpacing: Use one space before backslash.
  + 0

# String followed by backslash with no space (implicit concatenation)
msg = "Expected result "\
                        ^ Layout/LineContinuationSpacing: Use one space before backslash.
    "but found something else"

raise TypeError, "Argument must be a Binding, not "\
                                                   ^ Layout/LineContinuationSpacing: Use one space before backslash.
    "a #{scope.class.name}"

result = 'hello '\
                 ^ Layout/LineContinuationSpacing: Use one space before backslash.
    'world'

x = 'first'\
           ^ Layout/LineContinuationSpacing: Use one space before backslash.
    'second'

# Multiple spaces before backslash after string
msg2 = "hello"   \
              ^^^^ Layout/LineContinuationSpacing: Use one space before backslash.
    "world"

# Method call with backslash continuation, no space
expect { foo }.to\
                 ^ Layout/LineContinuationSpacing: Use one space before backslash.
  raise_error(ArgumentError)

# Multi-line condition, no space before backslash
if condition_a &&\
                 ^ Layout/LineContinuationSpacing: Use one space before backslash.
   condition_b
  do_something
end
