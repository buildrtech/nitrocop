# =~ in if condition
if x =~ /pattern/
   ^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
  do_something
end
# =~ in unless condition
do_something unless x =~ /re/
                    ^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
# !~ in if condition
if x !~ /pattern/
   ^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `!~` when `MatchData` is not used.
  do_something
end
# !~ in unless condition (modifier)
do_something unless x !~ /re/
                    ^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `!~` when `MatchData` is not used.
# .match() with regexp arg in if condition
if foo.match(/re/)
   ^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `match` when `MatchData` is not used.
  do_something
end
# .match() with string receiver in if condition
if "foo".match(re)
   ^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `match` when `MatchData` is not used.
  do_something
end
# .match() with regexp receiver in if condition
if /re/.match(foo)
   ^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `match` when `MatchData` is not used.
  do_something
end
# .match() with symbol receiver in if condition
if :foo.match(re)
   ^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `match` when `MatchData` is not used.
  do_something
end
# .match() with position arg in if condition
if "foo".match(re, 1)
   ^^^^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `match` when `MatchData` is not used.
  do_something
end
# === with regexp in if condition
if /re/ === foo
   ^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `===` when `MatchData` is not used.
  do_something
end
# === with regexp with flags in if condition
if /re/i === foo
   ^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `===` when `MatchData` is not used.
  do_something
end
# =~ in elsif condition
if cond
  do_something
elsif x =~ /pattern/
      ^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
  do_something2
end
# =~ in ternary
x =~ /pattern/ ? do_something : do_something2
^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
# case/when with regexp
case
when /re/ === str
     ^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `===` when `MatchData` is not used.
  do_something
end
# =~ in modifier if
do_something if x =~ /re/
                ^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
# MatchData used in different method - still flag
def foo
  if x =~ /re/
     ^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
    do_something
  end
end
def bar
  do_something($1)
end
# MatchData reference before the match - still flag
def baz
  do_something($1)
  if x =~ /re/
     ^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
    do_something2
  end
end
# Multiple matches in same method — first match has no MatchData use,
# second match does. First should be flagged, second should not.
def multi_match
  if x =~ /first/
     ^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
    do_something
  end
  if y =~ /second/
    do_something($1)
  end
end
# MatchData ref overridden by a later match — first match should still flag
def overridden_match
  if x =~ /re/
     ^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
    do_something
  end
  if y =~ /other/
    do_something2
  end
  puts $1
end
# Top-level code: multiple matches, only some use MatchData
message = message.sub(/re/, '') if message =~ /pattern/
                                   ^^^^^^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
# =~ in class body, MatchData is in a method (different scope) — flag
class Checker
  if name =~ /pattern/
     ^^^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
    do_something
  end

  def check
    $1
  end
end
# =~ in module body, MatchData is in a method (different scope) — flag
module Validator
  if name =~ /pattern/
     ^^^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
    do_something
  end

  def validate
    $1
  end
end
# bare match() with regexp arg — no receiver (implicit self)
if match(/pattern/)
   ^^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `match` when `MatchData` is not used.
  do_something
end
# bare match() with string arg — no receiver
unless match("pattern")
       ^^^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `match` when `MatchData` is not used.
  do_something
end
# =~ where next non-condition match resets MatchData boundary
def boundary_test
  if prefix =~ /^\s*$/
     ^^^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
    new_offset = offset
  else
    new_offset = offset
    prefix =~ /^(\s*)[^\s].*$/
    len = $1 ? $1.length : 0
    new_offset += len
  end
end
# Sequential modifier-if: both first matches have no MatchData refs in their
# ranges (between current match and next match). Third match has $1 in its body.
def sequential_modifier
  raise "no db" if out =~ /^ERROR.+: No database selected/
                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
  raise "no match" if out =~ /^ERROR.+: Connection refused/
                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
  if out =~ /^ERROR.+Unknown database '(.+)'/
    raise "missing #{$1}"
  end
end
# case/when: second when has no MatchData use — flag it
case
when line =~ /^ORIGINAL ?([\w\s]+)$/
  name = $~[1].strip
when line =~ /^EXPECTED$/
     ^^^^^^^^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
  current[:expected] = ""
when line =~ /^PENDING$/
     ^^^^^^^^^^^^^^^^^^^^^ Performance/RegexpMatch: Use `match?` instead of `=~` when `MatchData` is not used.
  current[:pending] = true
end
