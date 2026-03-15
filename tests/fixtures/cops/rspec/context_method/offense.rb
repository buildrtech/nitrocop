context '.foo_bar' do
        ^^^^^^^^^^ RSpec/ContextMethod: Use `describe` for testing methods.
end

context '#foo_bar' do
        ^^^^^^^^^^ RSpec/ContextMethod: Use `describe` for testing methods.
end

context '#another_method' do
        ^^^^^^^^^^^^^^^^^ RSpec/ContextMethod: Use `describe` for testing methods.
end

context "#to_boolean for #{value.inspect}" do
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ContextMethod: Use `describe` for testing methods.
end

context ".to_integer for #{value.inspect}" do
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ContextMethod: Use `describe` for testing methods.
end

context "#method_#{name}" do
        ^^^^^^^^^^^^^^^^^ RSpec/ContextMethod: Use `describe` for testing methods.
end
