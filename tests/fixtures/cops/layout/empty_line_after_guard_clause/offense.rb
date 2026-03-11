def foo
  return if need_return?
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  bar
end

def baz
  raise "error" unless valid?
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  do_work
end

def quux
  return unless something?
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  process
end

def notice_params
  return @notice_params if @notice_params
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  @notice_params = params[:data] || request.raw_post
  if @notice_params.blank?
    fail ParamsError, "Need a data params in GET or raw post data"
  end
  ^^^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  @notice_params
end

# Guard clause followed by bare raise (not a guard line)
def exception_class
  return @exception_class if @exception_class
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  raise NotImplementedError, "error response must define #exception_class"
end

# Guard clause with `and return` form
def with_and_return
  render :foo and return if condition
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  do_something
end

# Guard clause with `or return` form
def with_or_return
  render :foo or return if condition
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  do_something
end

# Guard clause before `begin` keyword
def guard_before_begin
  return another_object if something_different?
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  begin
    bar
  rescue SomeException
    baz
  end
end

# Guard clause followed by rubocop:disable comment (no blank line between)
def guard_then_rubocop_disable
  return if condition
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  # rubocop:disable Department/Cop
  bar
  # rubocop:enable Department/Cop
end

# Guard clause followed by rubocop:enable comment then code (no blank after enable)
def guard_then_rubocop_enable
  # rubocop:disable Department/Cop
  return if condition
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  # rubocop:enable Department/Cop
  bar
end
