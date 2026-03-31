# Single-line if with semicolon
if foo
 bar end

if foo
 bar else baz end

if condition
 do_something end

# Multi-line if with semicolon after condition (body on next line)
if true
  do_something
end

# Unless with semicolon, multi-line
unless done
  process
end

# Multi-line if with semicolon and parenthesized condition
if (97 <= cc && cc <= 122)
  return true
end

# Trailing semicolon with simple parenthesized condition
if (octets)
  index = process(octets, result, index)
end

# Nested if with semicolon inside parent if with semicolon (RuboCop ignore_node)
# Only the outer if is flagged; inner if is suppressed via part_of_ignored_node?
if is_real?
  if @re>=0; return foo
  else return bar
  end
end

# Nested if with semicolon inside elsif with semicolon
# Only the outer if is flagged; nested ifs are suppressed
if other.kind_of?(Quaternion)
 ((self.log)*other).exp
elsif other.kind_of?(Integer);
  if other==0; return One
  elsif other>0; x = self
  end
end

# if with semicolon inside case else (not an if's else) — should be flagged
# The `else` here belongs to `case`, not to an `if` node, so
# `node.parent&.if_type?` is false in RuboCop.
case tt
when :slash then slt = tt
else if at
 zt = tt; else; at = tt; end
end

