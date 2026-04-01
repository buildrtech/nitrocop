x = [
  :a,
  :b,
  :c
]

y = [
  1,
  2,
  3
]

z = [
  :foo,
  :bar,
  :baz
]

# Rescue exception lists — multiple exceptions on same line in multi-line rescue
begin
  something
rescue FooError,
       BarError,
       BazError
  retry
end

begin
  something
rescue FooError,
       BarError,
       BazError,
       QuxError
  retry
end

# Implicit array in multi-assignment (no brackets)
a, b, c =
  val1,
  val2,
  val3

# Method call with implicit array args (e.g. config.cache_store=)
config.cache_store = :redis_cache_store,
                     {
  url: "redis://localhost:6379/1",
  expires_in: 90
}

# Constant assignment with implicit array
ITEMS = :scan,
        :skip,
  :match
