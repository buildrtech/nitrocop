# rubocop:disable Layout/SpaceAroundOperators
x =   0
# rubocop:enable Layout/SpaceAroundOperators
# Some other code
# rubocop:disable Layout
x =   0
# rubocop:enable Layout
# Some other code
x = 1 # rubocop:disable Layout/LineLength
y = 2

# Directives inside heredocs should not be detected
code = <<~RUBY
  # rubocop:disable Layout/LineLength
  very_long_line = 1
RUBY
puts code

# Directives can include an inline explanation after the cop name.
# rubocop:disable Development/NoEvalCop This eval takes static inputs at load-time
eval(source)
# rubocop:enable Development/NoEvalCop

# `enable all` should close individual cop disables
# rubocop:disable Metrics/MethodLength
def long_method
  x = 1
end
# rubocop:enable all

# `enable all` should close department-level disables
# rubocop:disable Layout
x =   0
# rubocop:enable all

# `enable all` should close multiple individual disables at once
# rubocop:disable Metrics/MethodLength
# rubocop:disable Style/FrozenStringLiteralComment
x = 1
y = 2
# rubocop:enable all

# `disable all` followed by `enable all`
# rubocop:disable all
x = 1
# rubocop:enable all

# Trailing explanation with `--` marker should not create phantom cops
# rubocop:disable Style/Foo -- use bar, baz instead
x = 1
# rubocop:enable Style/Foo

# Trailing explanation after `--` with comma should not split into cop names
# rubocop:disable Metrics/MethodLength -- long method, needs refactoring later
def calculate
  x = 1
end
# rubocop:enable Metrics/MethodLength

# Invalid token before a department disable should not leave a phantom department open
# rubocop:disable /BlockLength, Metrics/
RSpec.describe Example do
  it "works" do
    expect(result).to be_truthy
  end
end
