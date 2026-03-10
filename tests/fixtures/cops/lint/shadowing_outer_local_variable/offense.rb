def some_method
  foo = 1
  puts foo
  1.times do |foo|
              ^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `foo`.
  end
end
def other_method
  foo = 1
  puts foo
  1.times do |i; foo|
                 ^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `foo`.
    puts foo
  end
end
def method_arg(foo)
  1.times do |foo|
              ^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `foo`.
  end
end
# Nested block: inner block param shadows outer block param
def nested_shadow
  items.each do |slug|
    slug.children.map! { |slug| slug.upcase }
                          ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `slug`.
  end
end
# Destructured block param shadows method arg
def theme_svgs(theme_id)
  sprites.map do |(theme_id, upload_id)|
                   ^^^^^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `theme_id`.
    [theme_id, upload_id]
  end
end
# Block inside if still shadows outer method arg
def some_method(env)
  if some_condition
    pages.each do |env|
                   ^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `env`.
      do_something(env)
    end
  end
end
# Block param shadowing inside if/unless branch still flags
def handler(name)
  if block_given?
    items.each do |name|
                   ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `name`.
      yield name
    end
  end
end
# Same branch of same if condition node
def some_method
  if condition?
    foo = 1
    puts foo
    bar.each do |foo|
                 ^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `foo`.
    end
  else
    bar.each do |foo|
    end
  end
end
# Splat block param shadows outer
def some_method
  foo = 1
  puts foo
  1.times do |*foo|
              ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `foo`.
  end
end
# Block block param shadows outer
def some_method
  foo = 1
  puts foo
  proc_taking_block = proc do |&foo|
                               ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `foo`.
  end
  proc_taking_block.call do
  end
end

# Post parameter shadows in inner block
def configure(*items, tail)
  jobs.each do |tail|
                ^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `tail`.
    puts tail
  end
end

# Keyword rest parameter shadows in inner block
def configure(**options)
  handler = proc do |**options|
                     ^^^^^^^^^ Lint/ShadowingOuterLocalVariable: Shadowing outer local variable - `options`.
    options
  end
  handler.call
end
