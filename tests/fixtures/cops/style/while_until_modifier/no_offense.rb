x -= 1 while x > 0
process_next until done
while x > 0
  x -= 1
  puts x
end
until done
  step1
  step2
end
x = 10
while some_very_long_condition_name && another_very_long_condition_name && yet_another_very_long_condition_name
  do_something_with_a_very_long_body_that_exceeds_line_length
end
while (chunk = file.read(1024))
  io.write(chunk)
end
while (node = node.receiver)
  return true if node.heredoc?
end
# Multi-line conditions should not suggest modifier form
while ancestors.any? &&
      !issue.is_descendant_of?(ancestors.last)
  ancestors.pop
end
until starting >= max ||
      line_left[starting] != line_right[starting]
  starting += 1
end
while valid?(item) &&
      item.active? &&
      item.visible?
  process(item)
end
# Comment inside body makes it non-eligible (contains_comment? on body range)
while raw_connection.next_result
  # just to make sure that all queries are finished
  _result = raw_connection.store_result
end
# Comment on same line as body statement
while relation.delete_all == 100
  true # loop
end
# Comment on end line makes it non-eligible
while condition
  do_something
end # some comment
# Code after end keyword (trailing modifier) — must be included in length calculation
while tarr.first[1] == "[None]"
  tarr.push(tarr.shift)
end unless tarr.blank? || (tarr.first[1] == "[None]" && tarr.last[1] == "[None]")
# Comment on first line AND code after end — skip entirely
[
  1, while foo # bar
       baz
     end, 3
]
