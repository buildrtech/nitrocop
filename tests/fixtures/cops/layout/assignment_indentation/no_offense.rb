x =
  1

y = 2

CONST =
  "hello"

# Conditional assignments - RuboCop does not flag these
if upload =
     Upload.find_by(name: params[:name])
  process(upload)
end

if match =
     url.match(%r{/foo/bar})
  handle(match)
end

while item =
        queue.pop
  process(item)
end

# Same-line RHS is always OK
x = if condition
      1
    end

# Properly indented embedded assignment
def foo
  if result =
       compute(value)
    result
  end
end

# Properly indented operator assignments
x +=
  1

y ||=
  "default"

# Properly indented setter/index assignments
result[:key] =
  hash_from_xml(data)

self.name =
  compute_name(input)

obj.attr ||=
  default_value
