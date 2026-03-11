def foo
  return if need_return?

  bar
end

def baz
  raise "error" unless valid?

  do_work
end

def quux
  return unless something?

  process
end

def notice_params
  return @notice_params if @notice_params

  @notice_params = params[:data] || request.raw_post
  if @notice_params.blank?
    fail ParamsError, "Need a data params in GET or raw post data"
  end

  @notice_params
end

# Guard clause followed by bare raise (not a guard line)
def exception_class
  return @exception_class if @exception_class

  raise NotImplementedError, "error response must define #exception_class"
end

# Guard clause with `and return` form
def with_and_return
  render :foo and return if condition

  do_something
end

# Guard clause with `or return` form
def with_or_return
  render :foo or return if condition

  do_something
end

# Guard clause before `begin` keyword
def guard_before_begin
  return another_object if something_different?

  begin
    bar
  rescue SomeException
    baz
  end
end

# Guard clause followed by rubocop:disable comment (no blank line between)
def guard_then_rubocop_disable
  return if condition

  # rubocop:disable Department/Cop
  bar
  # rubocop:enable Department/Cop
end

# Guard clause followed by rubocop:enable comment then code (no blank after enable)
def guard_then_rubocop_enable
  # rubocop:disable Department/Cop
  return if condition

  # rubocop:enable Department/Cop
  bar
end
