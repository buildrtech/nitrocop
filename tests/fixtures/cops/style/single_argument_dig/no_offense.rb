{ key: 'value' }[:key]
[1, 2, 3][0]
{ key1: { key2: 'value' } }.dig(:key1, :key2)
[1, [2, [3]]].dig(1, 1)
keys = %i[key1 key2]
{ key1: { key2: 'value' } }.dig(*keys)

# dig with argument forwarding should not be flagged
def fetch_value(...)
  data.dig(...)
end

# dig with anonymous rest forwarding
def fetch_value(*)
  data.dig(*)
end

# dig with anonymous keyword forwarding
def fetch_value(**)
  data.dig(**)
end

# dig with anonymous block argument forwarding
def fetch_value(&)
  data.dig(&)
end
