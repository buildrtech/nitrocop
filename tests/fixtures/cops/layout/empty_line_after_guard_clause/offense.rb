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

# Guard followed by rubocop:disable directive (not an allowed directive)
def guard_with_disable_directive
  return if need_return?
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  # rubocop:disable Metrics/AbcSize

  bar
  # rubocop:enable Metrics/AbcSize
end

# Guard clause followed by regular comment then blank line then code (FP fix)
# RuboCop checks the immediate next line, not the first code line
def guard_comment_then_blank
  return if condition
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  # This is a regular comment

  bar
end

# Guard clause with heredoc argument (FN fix)
def guard_with_heredoc
  raise ArgumentError, <<-MSG unless path
    Must be called with mount point
  MSG
  ^^^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  bar
end

# Guard clause with squiggly heredoc
def guard_with_squiggly_heredoc
  raise ArgumentError, <<~MSG unless path
    Must be called with mount point
  MSG
  ^^^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  bar
end

# Ternary guard clause
def ternary_guard
  puts 'some action happens here'
rescue => e
  a_check ? raise(e) : other_thing
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  true
end

# FN fix: guard clause followed by if block with multi-line raise
def guard_then_if_multiline_raise
  return if !argv
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  if argv.empty? || argv.length > 2
    raise Errors::CLIInvalidUsage,
      help: opts.help.chomp
  end
end

# FN fix: guard followed by if-block with modifier-form return (NOT a guard per RuboCop)
def guard_then_if_modifier_return
  return unless doc.blocks?
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  if (first_block = doc.blocks[0]).context == :preamble
    return unless (first_block = first_block.blocks[0])
  elsif first_block.context == :section
    return
  end
end

# FN fix: block-form guard `unless..raise..end` followed by non-guard code
def block_guard_then_nonguard
  unless valid?(level)
    raise "invalid"
  end
  ^^^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  if logger.respond_to?(:add)
    logger.add(level, message)
  else
    raise "invalid logger"
  end
end

# FN fix: simple guard clause patterns missing empty line
def ask_user(question)
  return true if args['-y']
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  $stderr.print question
  $stdin.gets =~ /\Aye?s?\Z/i
end

def format_time(time)
  return '' unless time.respond_to?(:strftime)
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  time.strftime('%H:%M:%S')
end

def parse_entry(entry)
  next unless entry.end
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  entry.update :sheet => "_value"
end

# FN fix: guard followed by bare return (not a guard line) where the return
# line happens to contain `?` in a method name — should NOT be suppressed
# by ternary guard check
def generated_thrift?
  return false unless THRIFT_EXTENSIONS.include?(extname)
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  return lines.first(6).any? { |l| l.include?("Autogenerated") }
end

# FN fix: multiple consecutive `return false unless` followed by bare return
def vcr_cassette?
  return false unless extname == '.yml'
  return false unless lines.count > 2
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  return lines[-2].include?("recorded_with: VCR")
end

# FN fix: guard followed by bare return with no `?` but still not a guard line
def check_overflow(av, bv)
  reset_state()
  return  unless av.kind_of?(Integer) && bv.kind_of?(Integer)
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  @expected = compute(av, bv)
end

# FN fix: return nil if followed by code (bare return with `if`)
def test_login
  s = Etc.getlogin
  return if s == nil
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  assert(s.is_a?(String), "must return a String or nil")
end

# FN fix: bare return with ternary value is not a ternary guard sibling
def namespace
  return 'Object' if duck_type?
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  return (name == 'Class' || name == 'Module') && !subtypes.empty? ? subtypes.first.name : name
end

# FN fix: next line contains an embedded guard inside a block, not a guard sibling
def update?(cookies)
  return true if cached.empty?
  ^ Layout/EmptyLineAfterGuardClause: Add empty line after guard clause.
  cookies.each { |key, value| return true if previous_cookie(key) != value }
  false
end
