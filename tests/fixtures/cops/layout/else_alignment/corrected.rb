if foo
  bar
else
  baz
end

if foo
  bar
elsif qux
  baz
end

if alpha
  one
else
  two
end

value = if condition
          one
        else
          two
        end
result = if foo
  bar
         else
  baz
end

# case/when: else should align with `when`
case a
when b
  c
when d
  e
else
  f
end

# case/when: else indented too far
case code_type
when 'ruby'
  code_type
when 'erb'
  'ruby'
else
    'plain'
end

# case/in (pattern matching): else should align with `in`
case 0
in 0
  foo
in -1..1
  bar
in Integer
  baz
else
  qux
end

# begin/rescue/else: else should align with `begin`
begin
  something
rescue
  handling
else
  fallback
end

# def/rescue/else: else should align with `def`
def my_func
  puts 'hello'
rescue => e
  puts e
else
  puts 'ok'
end

# unless: else should align with `unless` keyword
unless condition
  one
else
  two
end

# unless assignment: else at col 0 should align with `unless` at col 11
response = unless identity
             service.call
           else
             other.call
end

# begin/rescue/else: else at column 0 should align with `begin`
def my_func
  begin
    puts 'error prone'
  rescue
    puts 'handling'
  else
    puts 'normal'
  end
end
