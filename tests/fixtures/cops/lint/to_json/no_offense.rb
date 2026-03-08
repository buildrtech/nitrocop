def to_json(*args)
  JSON.generate([x, y], *args)
end

def to_json(*_args)
  JSON.generate([x, y])
end

def to_json(options = {})
  '{}'
end

def to_s
  'hello'
end

# Singleton method definitions should not be flagged
obj = Object.new
def obj.to_json
  '{"role":"user"}'
end

def obj.to_json = '{"custom":"json"}'
