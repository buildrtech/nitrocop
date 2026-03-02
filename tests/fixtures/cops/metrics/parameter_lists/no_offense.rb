def short_params(a, b, c)
  a + b + c
end

def five_params(a, b, c, d, e)
  [a, b, c, d, e]
end

def no_params
  42
end

# Proc and lambda parameters are exempt
proc { |a, b, c, d, e, f| a }
->(a, b, c, d, e, f) { a }

# Block params under the limit are fine
data.each do |a, b, c|
  a
end

items.map do |a, b, c, d, e|
  a
end

# Struct.new initialize exemption
Struct.new(:one, :two, :three, :four, :five, :six) do
  def initialize(one:, two:, three:, four:, five:, six:)
  end
end

# ::Struct.new initialize exemption
::Struct.new(:one, :two, :three, :four, :five, :six) do
  def initialize(one:, two:, three:, four:, five:, six:)
  end
end

# Data.define initialize exemption
Data.define(:one, :two, :three, :four, :five, :six) do
  def initialize(one:, two:, three:, four:, five:, six:)
  end
end

# ::Data.define initialize exemption
::Data.define(:one, :two, :three, :four, :five, :six) do
  def initialize(one:, two:, three:, four:, five:, six:)
  end
end
