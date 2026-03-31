loop do
  work
end

loop do
  work
end

loop do
  break if done?
end

loop do
  work
end

loop do
  work
end

loop do
  work
end

# Variable assigned inside loop but NOT referenced after — still an offense
def no_ref_after
  loop do
    a = 43
    break
  end
end

# Variable assigned before loop — safe to convert, still an offense
def pre_assigned
  a = 0
  loop do
    a = 42
    break
  end
  puts a
end

# Instance variable assigned inside loop — safe, still an offense
def ivar_assign
  loop do
    @a = 42
    break
  end
  puts @a
end
