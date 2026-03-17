def foo
  return if need_return?

  bar
end

def baz
  return if something?
  return if something_different?

  bar
end

def quux
  raise "error" unless valid?

  do_work
end

def last_guard
  return if done?
end

def consecutive_with_embedded_return
  return if does_not_expire?
  requeue! && return if not_due_yet?

  notify_remote_voters_and_owner!
end

def consecutive_mixed_guards
  raise "error" unless valid?
  do_something || return if stale?

  process
end

# Comment between consecutive guard clauses is OK
def comment_between_guards
  return if first_condition?
  # This is a comment explaining the next guard
  return if second_condition?

  do_work
end

# Multiple comments between guards
def multi_comment_between_guards
  return unless valid_input?
  # First reason
  # Second reason
  return if already_processed?

  process
end

# Guard followed by multi-line if block containing return
def guard_then_multiline_if
  return if done?
  if complex_condition? && another_check?
    return
  end

  process
end

# Guard followed by multi-line unless block containing raise
def guard_then_multiline_unless
  return unless valid?
  unless authorized? && permitted?
    raise "unauthorized"
  end

  do_work
end

# Guard inside a block (embedded in larger expression)
def guard_in_block
  heredocs.each { |node, r| return node if r.include?(line_number) }
  nil
end

# Break inside a block (embedded in larger expression)
def break_in_block
  prev = items.each_cons(2) { |m, n| break m if n == item }
  between = prev.source_range
end

# Guard before end keyword
def guard_before_end
  return if something?
end

# Guard before else
def guard_before_else(x)
  if x > 0
    return if done?
  else
    work
  end
end

# Guard before rescue
def guard_before_rescue
  return if safe?
rescue => e
  handle(e)
end

# Guard before ensure
def guard_before_ensure
  return if cached?
ensure
  cleanup
end

# Guard before when
def guard_before_when(x)
  case x
  when :a
    return if skip?
  when :b
    work
  end
end

# Next in block followed by comment then next
def next_with_comments
  items.each do |item|
    next if item.blank?
    # Skip already processed items
    next if item.processed?

    process(item)
  end
end

# Raise with comment between guards
def raise_with_comments
  raise "error" unless condition_a?
  # Make sure condition_b is also met
  raise "other error" unless condition_b?

  run
end

# Guard with nocov directive followed by blank line
def guard_with_nocov
  # :nocov:
  return if condition?
  # :nocov:

  bar
end

# Guard clause is last statement before closing brace
def guard_before_closing_brace
  items.map do |item|
    return item if item.valid?
  end
end

# Guard followed by multiline if with return inside nested structure
def guard_then_nested_multiline_if
  return if line_length(line) <= max
  return if allowed_line?(line)
  if complex_config? && special_mode?
    return check_special(line)
  end

  register_offense(line)
end

# Multiple return false guards with comments
def multiple_return_false_guards
  return false unless first_check?
  # anonymous forwarding
  return true if special_case?
  return false unless second_check?
  return false unless third_check?

  name == value
end

# Guard with long comment block between guards
def long_comment_between_guards
  return false unless incoming? || outgoing?
  # For Chatwoot Cloud:
  #   - Enable indexing only if the account is paid.
  #   - The `advanced_search_indexing` feature flag is used only in the cloud.
  #
  # For Self-hosted:
  #   - Adding an extra feature flag here would cause confusion.
  return false if cloud? && !feature_enabled?('search')

  true
end

# Block-form if with guard clause followed by empty line — no offense
def block_guard_with_blank
  if params.blank?
    fail ParamsError, "Missing params"
  end

  process(params)
end

# Block-form if with guard clause at end of method — no offense
def block_guard_at_end
  if invalid?
    raise "invalid"
  end
end

# Block-form if with multiple statements — not a guard clause
def block_not_guard
  if condition?
    setup
    process
  end
  finalize
end

# Block-form if with multi-line guard statement — not a guard clause per RuboCop
# (guard_clause? requires single_line?)
def multiline_guard_in_block
  all.to_a.delete_if do |version|
    if item.respond_to?(:access)
      next item.user_id != user.id &&
        item.assigned_to != user.id &&
        (item.access == "Private")
    end
    next false
  end
end

# `and return` guard clause properly followed by blank line
def and_return_ok
  render :foo and return if condition

  do_something
end

# `or return` guard clause properly followed by blank line
def or_return_ok
  render :foo or return if condition

  do_something
end

# Guard with rubocop:enable followed by blank line
def guard_rubocop_enable_ok
  # rubocop:disable Department/Cop
  return if condition
  # rubocop:enable Department/Cop

  bar
end

# Multiple statements on same line with semicolon
def foo(item)
  return unless item.positive?; item * 2
end

# Guard before begin with blank line
def guard_before_begin_ok
  return another_object if something_different?

  begin
    bar
  rescue SomeException
    baz
  end
end

# Non-guard modifier if (not a guard clause)
def normal_modifier_if
  foo += 1 if need_add?
  foobar
end

# Guard clause with heredoc argument followed by blank line
def guard_heredoc_ok
  raise ArgumentError, <<-MSG unless path
    Must be called with mount point
  MSG

  bar
end

# Guard clause with squiggly heredoc followed by blank line
def guard_squiggly_heredoc_ok
  raise ArgumentError, <<~MSG unless path
    Must be called with mount point
  MSG

  bar
end

# Guard clause with heredoc in condition followed by blank line
def guard_heredoc_condition_ok
  return true if <<~TEXT.length > bar
    hi
  TEXT

  false
end

# Guard clause with heredoc and chained calls
def guard_heredoc_chained_ok
  raise ArgumentError, <<~END.squish.it.good unless guard
    A multiline message
    that will be squished.
  END

  return_value
end

# Ternary without guard clause - not flagged
def ternary_non_guard
  x = condition ? value_a : value_b
  do_something
end

# Guard clause followed by whitespace-only blank line (spaces)
# RuboCop treats whitespace-only lines as blank
def guard_whitespace_blank_spaces
  return false unless request&.fullpath&.start_with?(callback_path)
      
  # Try request.origin first, then fallback to referer.
  origin = request.origin
end

# Guard clause followed by whitespace-only blank line (tab)
def guard_whitespace_blank_tab
  raise ActiveRecord::RecordNotFound unless record.present?

  process(record)
end

# Consecutive guard clauses with line continuation (backslash)
def consecutive_guards_with_continuation
  raise ArgumentError, "invalid method" \
    unless method == 'dns'
  raise ArgumentError, "a non-empty list is required" \
    if servers.empty?
end

# Multiple consecutive guards with line continuation
def multiple_guards_continuation
  raise ArgumentError, "method should be a symbol" \
    unless method.is_a?(Symbol)
  raise ArgumentError, "uri should be a string" \
    unless uri.is_a?(String)
  raise ArgumentError, "body should be a string" \
    if body && !body.is_a?(String)
  raise ArgumentError, "headers should be a hash" \
    if headers && !headers.is_a?(Hash)
end

# Guard with line continuation followed by non-guard with blank line
def guard_continuation_then_blank
  raise ArgumentError, "invalid input" \
    unless valid?

  process
end

# Guard with line continuation at end of method
def guard_continuation_at_end
  raise ArgumentError, "missing config" \
    unless config.present?
end

# Guard with string concatenation continuation
def guard_string_concat_continuation
  raise "Must specify the file to " + \
    "convert to the new model" if filename.nil?
  raise "File does not " + \
    "exist: #{filename}" unless File.exist?(filename)
end

# Guard with multi-line return value followed by modifier
def guard_multiline_return_value
  return {
    status: "err",
    error: "Invalid input."
  }.to_json if !info
  return {
    status: "err",
    error: "Wrong ID."
  }.to_json if not get_item(id)
end

# Guard with multi-line return string followed by code
def guard_multiline_return_string
  return "
    * navigate
  " if options[:task] == []
  prefix = "open" if options[:task] == ["navigate"]
end

# Guard with multi-line raise (args on next line)
def guard_multiline_raise_args
  raise ArgumentError,
    "msg here" unless condition
  raise BadError,
    "Response is empty." if raw_text.blank?
end

# Guard with fail and line continuation
def guard_fail_continuation
  fail "Association defined for a second time. " \
       "Associations can only be defined once" if duplicate?(name)
  associations[name] = object
end

# Guard with parenthesized multi-line condition
def guard_paren_multiline_condition
  raise ArgumentError, "invalid interval" if (
      discovery.key?('interval') &&
      !(discovery['interval'].is_a?(Numeric) &&
      discovery['interval'] >= 0)
    )
  raise ArgumentError, "missing host" \
    unless discovery['hosts']
end

# Guard followed by comment, blank line, then another guard
def guard_comment_blank_guard
  next if file =~ /pattern_a/ && VERSION <= Gem::Version.new('1.7.25')
  # EMXIF

  # FIXME: Remove when we stop testing old version
  next if file =~ /pattern_b/ && VERSION <= Gem::Version.new('1.7.13')
end
