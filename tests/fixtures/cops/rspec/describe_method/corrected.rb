describe Some::Class, '#nope' do
end

describe MyClass, '#incorrect_usage' do
end

describe AnotherClass, '#something else' do
end

# String first argument (not a constant) — RuboCop still checks the second arg
RSpec.describe "Boards", "#Creating a view from a Global Context" do
end

RSpec.describe "Calendars", "#index" do
end

# Method call first argument — still checks second arg
describe Puppet::Type.type(:package), "#when choosing a default provider" do
end
