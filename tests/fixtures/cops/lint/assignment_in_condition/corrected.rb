if (x = 1)
  do_something
end

while (y = gets)
  process(y)
end

until (z = calculate)
  retry_something
end

if (@test = 10)
end

if (@@test = 10)
end

if ($test = 10)
end

if (TEST = 10)
end

if test == 10 || (foobar = 1)
end

if (test.method = 10)
end

if (test&.method = 10)
end

if (a[3] = 10)
end

do_something if (x = 1)

do_something while (y = gets)

unless (x = 1)
  do_something
end

if (foo == bar && (test = 10))
end

if (foo == bar || (test = 10))
end

foo { x if (y = z) }

raise StandardError unless (foo ||= bar) || (a = b)

if (Foo::Bar = 1)
end

while (Foo::Bar = fetch_data)
end

unless (Module::Config = load_config)
  apply_defaults
end

if (Foo::Bar = 1 || baz)
end

# Assignment inside begin..end used as part of an if condition
if valid? && begin
  (result = compute_value)
  result.present?
end
  use(result)
end

# Assignment inside begin..end used as while condition
while begin (item = queue.shift); item end
  process(item)
end

# Assignment in when condition (bare case used as elsif condition)
if false
elsif case
when (match = scan(/foo/))
  process(match)
end
end

# Assignment inside case/when body within an if condition
if (case kind
      when :special
        (found = lookup(kind))
      else
        false
    end)
  use(found)
end

# Assignment inside begin/rescue used as condition
return true if check? && begin
  (data = parse(input))
  data.valid?
rescue StandardError
  false
end

# Assignment inside rescue modifier in condition
return nil unless valid? || begin
  (uri = URI.parse(route) rescue nil)
  uri.present?
end
