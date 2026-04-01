def test
  return unless something
  work
end

def test
  return if something
  work
end

def test
  other_work
  return unless something
  work
end

def test
  other_work
  return if something
  work
end
