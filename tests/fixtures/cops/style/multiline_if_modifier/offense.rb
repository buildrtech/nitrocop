{
^ Style/MultilineIfModifier: Favor a normal if-statement over a modifier clause in a multiline statement.
  result: run
} if cond

{
^ Style/MultilineIfModifier: Favor a normal unless-statement over a modifier clause in a multiline statement.
  result: run
} unless cond

[
^ Style/MultilineIfModifier: Favor a normal if-statement over a modifier clause in a multiline statement.
  1,
  2,
  3
] if condition

raise "Value checking error" \
^ Style/MultilineIfModifier: Favor a normal if-statement over a modifier clause in a multiline statement.
    ' for attribute' if value.nil?

raise "Type checking error" \
^ Style/MultilineIfModifier: Favor a normal unless-statement over a modifier clause in a multiline statement.
  "for attribute" unless acceptable

fail "Association defined" \
^ Style/MultilineIfModifier: Favor a normal if-statement over a modifier clause in a multiline statement.
     "a second time" if duplicate?(name)

log_error("Returned nil body. " \
^ Style/MultilineIfModifier: Favor a normal if-statement over a modifier clause in a multiline statement.
          "Probably wanted empty string?") if @response.body.nil?

help! "You do not have permission" \
^ Style/MultilineIfModifier: Favor a normal unless-statement over a modifier clause in a multiline statement.
      'Perhaps try sudo.' unless File.writable?(dir)

# Explicit + operator with backslash continuation
raise "Must specify backup file to " + \
^ Style/MultilineIfModifier: Favor a normal if-statement over a modifier clause in a multiline statement.
  "convert to the new model" if filename.nil?

# Assignment with backslash continuation
@batch_size ||= \
^ Style/MultilineIfModifier: Favor a normal if-statement over a modifier clause in a multiline statement.
  ENV['BATCH_SIZE'].to_i if environment['BATCH_SIZE']

# Multiple backslash continuations (3+ lines)
raise StandardError, "spoofing attack?! " \
^ Style/MultilineIfModifier: Favor a normal if-statement over a modifier clause in a multiline statement.
  "CLIENT=#{@req.client_ip} " \
  "FORWARDED=#{@req.x_forwarded_for}" if @req.forwarded_for.any?

# Nested modifier ifs: only the outermost is flagged (RuboCop's ignore_node behavior)
# The class...end is the body of the outer modifier if, so only the outer is flagged.
# The inner def...end if/unless inside it are ignored.
class TestDigestHash < Test::Unit::TestCase
^ Style/MultilineIfModifier: Favor a normal if-statement over a modifier clause in a multiline statement.
  LIB = "digest/md5"
  ALGO = Digest::MD5

  def type_name
    self.to_a[9]
  end unless method_defined?(:type_name)

  def param_symbols
    ary = self.to_a
    locals = ary.to_a[10]
    locals[0...3]
  end unless method_defined?(:param_symbols)
end if defined?(Digest)
