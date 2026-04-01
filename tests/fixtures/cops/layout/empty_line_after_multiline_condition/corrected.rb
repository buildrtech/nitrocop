# Block if with multiline condition
if foo &&
   bar

  do_something
end

# Block unless with multiline condition
unless foo &&
       bar

  do_something
end

# Block while with multiline condition
while foo &&
      bar

  do_something
end

# Block until with multiline condition
until foo ||
      bar

  do_something
end

# elsif with multiline condition
if condition
  do_something
elsif multiline &&
   condition

  do_something_else
end

# Modifier if with multiline condition and right sibling
do_something if multiline &&
                condition

do_something_else

# case/when with multiline condition
case x
when foo,
    bar
  do_something
end

# rescue with multiline exceptions
begin
  do_something
rescue FooError,
  BarError
  handle_error
end

# Modifier while (non-begin form) with multiline condition — always check
nil while
    foo &&
    bar

do_something

# Block if with multiline do..end block condition
if items.find do |item|
     item.ready?
   end

  process
end

# Modifier unless as sole body of if-branch with else (right_sibling = else body in AST)
if record
  redirect_to(record.url) unless params == { 'controller' => 'frontend', 'action' => 'show',
                                             'slug' => record.slug }

else
  render html: 'Not found.'
end

# Modifier if as sole body of if-branch with else
if record
  update_value(record) if params != { 'controller' => 'frontend', 'action' => 'show',
                                      'slug' => record.slug }

else
  render html: 'Not found.'
end

# Modifier unless with multiline args inside if-branch with elsif
if kind == :text
  return render_403 unless can_read_protocol?(protocol) ||
                           can_write_protocol?(protocol)

elsif kind == :result
  return render_403 unless can_read_result?(parent)
end

# Modifier if with line continuation inside begin/rescue
begin
  state_id = :unknown if !state_id && \
    host_ready? && !host_comm.ready?

rescue StandardError
  state_id = :unknown
end
