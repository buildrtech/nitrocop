if cond
{
  result: run
}
end

unless cond
{
  result: run
}
end

if condition
[
  1,
  2,
  3
]
end

if value.nil?
raise "Value checking error" \
    ' for attribute'
end

unless acceptable
raise "Type checking error" \
  "for attribute"
end

if duplicate?(name)
fail "Association defined" \
     "a second time"
end

if @response.body.nil?
log_error("Returned nil body. " \
          "Probably wanted empty string?")
end

unless File.writable?(dir)
help! "You do not have permission" \
      'Perhaps try sudo.'
end

# Explicit + operator with backslash continuation
if filename.nil?
raise "Must specify backup file to " + \
  "convert to the new model"
end

# Assignment with backslash continuation
if environment['BATCH_SIZE']
@batch_size ||= \
  ENV['BATCH_SIZE'].to_i
end

# Multiple backslash continuations (3+ lines)
if @req.forwarded_for.any?
raise StandardError, "spoofing attack?! " \
  "CLIENT=#{@req.client_ip} " \
  "FORWARDED=#{@req.x_forwarded_for}"
end

# Nested modifier ifs: only the outermost is flagged (RuboCop's ignore_node behavior)
# The class...end is the body of the outer modifier if, so only the outer is flagged.
# The inner def...end if/unless inside it are ignored.
if defined?(Digest)
class TestDigestHash < Test::Unit::TestCase
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
end
end
