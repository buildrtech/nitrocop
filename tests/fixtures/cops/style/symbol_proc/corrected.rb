foo.map(&:to_s)

bar.select(&:valid?)

items.reject(&:nil?)

# Ruby 3.4 it-block patterns
items.map(&:to_s)

records.select(&:visible)

servers.any?(&:needs_recycling?)

# Numbered parameter _1 patterns
items.map(&:to_s)

records.select(&:active?)

# super blocks (FN cases)
super(&:call_on_yielded)

super(headers) do |format|
  format.mjml
end
