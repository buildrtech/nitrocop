def to_json(*_args)
  JSON.generate([x, y])
end

class Foo
  def to_json(*_args)
    '{}'
  end
end

class Bar
  def to_json(*_args)
    JSON.generate(data)
  end
end
