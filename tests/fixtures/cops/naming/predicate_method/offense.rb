# Method returning comparison should end with ?
def foo
    ^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  bar == baz
end

# Method returning negation should end with ?
def checks_negation
    ^^^^^^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  !x
end

# Method returning predicate call should end with ?
def checks_predicate
    ^^^^^^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  bar?
end

# Method returning true should end with ?
def returns_true
    ^^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  true
end

# Method returning false should end with ?
def returns_false
    ^^^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  false
end

# Predicate method returning non-boolean literal
def bad_predicate?
    ^^^^^^^^^^^^^^ Naming/PredicateMethod: Non-predicate method names should not end with `?`.
  5
end

# Predicate method returning string literal
def string_pred?
    ^^^^^^^^^^^^ Naming/PredicateMethod: Non-predicate method names should not end with `?`.
  'hello'
end

# Predicate method returning nil literal
def nil_pred?
    ^^^^^^^^^ Naming/PredicateMethod: Non-predicate method names should not end with `?`.
  nil
end

# Class method returning boolean
def self.class_check
         ^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  x > y
end

# Predicate with bare return and ||= assignment (assignment is not call_type)
def self.enterprise?
         ^^^^^^^^^^^ Naming/PredicateMethod: Non-predicate method names should not end with `?`.
  return if ENV.fetch('DISABLE_ENTERPRISE', false)
  @enterprise ||= root.join('enterprise').exist?
end

# Explicit return with compound and-expression (return a? && b?)
def has_flag
    ^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  return active? && enabled?
end

# Explicit return with compound or-expression (return x > 0 || y > 0)
def is_valid
    ^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  return x > 0 || y > 0
end

# Explicit return with case expression
def has_role
    ^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  case kind
  when :admin then admin?
  when :member then member?
  else false
  end
end

# Nested def inside singleton class inside another method
def setup
  class << (@object = Object.new)
    def callback
        ^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
      true
    end
  end
end

# Method ending with ? returning nil from early return, and call+block as implicit return
# In RuboCop, call+block is NOT call_type?, so conservative skip doesn't apply
def fragment_exist?(key, options = nil)
    ^^^^^^^^^^^^^^^^ Naming/PredicateMethod: Non-predicate method names should not end with `?`.
  return unless cache_configured?
  instrument_fragment_cache(:exist_fragment?, key) do
    cache_store.exist?(key, options)
  end
end

# Non-predicate returning block_argument predicate call
def self.auto_bump_topic!
         ^^^^^^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  Category.shuffle.any?(&:auto_bump_topic!)
end

# Predicate with modifier-if assignment and no else — implicit nil is non-boolean literal
def valid_event_payload?
    ^^^^^^^^^^^^^^^^^^^^ Naming/PredicateMethod: Non-predicate method names should not end with `?`.
  @channel = Channel::Line.find_by(line_channel_id: @params[:line_channel_id]) if @params[:line_channel_id]
end

# Predicate with opaque branch value and no else — implicit nil is non-boolean literal
def instance_type?(type)
    ^^^^^^^^^^^^^^ Naming/PredicateMethod: Non-predicate method names should not end with `?`.
  if type.is_a?(Types::Name::Instance)
    type
  end
end

# Method returning parenthesized comparison should end with ?
def success_from(response)
    ^^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  (response[:response_code] == '0')
end

# Method returning parenthesized negation should end with ?
def no_errors(response)
    ^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  (!response['error'])
end

# Method returning parenthesized boolean chain should end with ?
def check_both
    ^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  (x.present? && y.present?)
end

# Method returning parenthesized or-chain should end with ?
def check_either
    ^^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  (a > b || c < d)
end

# Method returning parenthesized negation of predicate should end with ?
def check_not
    ^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  (!disabled?)
end

# Predicate with explicit nil return and parenthesized body in multi-statement method.
# The parens body is Opaque (not unwrapped in multi-statement context), so no boolean
# values exist. The nil from `return nil` is NonBooleanLiteral → offense.
def archive?(filename)
    ^^^^^^^^ Naming/PredicateMethod: Non-predicate method names should not end with `?`.
  return nil unless filename
  archive_type = get_archive_type(filename)
  (archive_type.include?("tar") || archive_type.include?("gzip") || archive_type.include?("zip"))
end

# Predicate method with yield and no else — yield is NOT call_type? in RuboCop,
# so conservative mode does not skip. Implicit nil is non-boolean literal.
def nil?(value)
    ^^^^ Naming/PredicateMethod: Non-predicate method names should not end with `?`.
  yield if value.nil? && block_given?
end

# Predicate method with yield in unless — implicit nil from missing else
def unfiltered?
    ^^^^^^^^^^^ Naming/PredicateMethod: Non-predicate method names should not end with `?`.
  s = url_params("substr")
  yield unless s && s.size > 0
end

# Ternary with parenthesized comparison in branch should flag
def check_result(response)
    ^^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  response[:status] ? (response[:code] == '0') : false
end

# If/else with both branches parenthesized boolean expressions
def matches_status(response)
    ^^^^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  if response['result']
    (response['status'] == 'ok' && response['result'] == 'ok')
  else
    (response['status'] == 'ok' || response['code'] == '1')
  end
end

# Explicit return with parenthesized comparison (Parser strips :begin for return args)
def hang_closing
    ^^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  i = get_value.to_i
  return (i != 0)
end

# If/elsif with parenthesized comparison in one branch, all boolean
def success_from(name, response)
    ^^^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  if !response
    false
  elsif name == 'refunds'
    response.include?('status')
  elsif response.include?('status')
    (response['status'] == 'succeeded')
  elsif name == 'cards'
    !!response['id']
  else
    false
  end
end

# If/elsif returning booleans but no else — RuboCop's IfNode#branches
# flattens elsif chains but EXCLUDES nil for missing else on inner elsifs,
# and extract_conditional_branches only pushes nil if node.else_branch is nil
# (which it isn't when there's an elsif, since elsif IS the else_branch).
def to_boolean
    ^^^^^^^^^^ Naming/PredicateMethod: Predicate method names should end with `?`.
  if ["true", true].include? value
    true
  elsif ["false", false].include? value
    false
  end
end
