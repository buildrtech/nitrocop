class User < ApplicationRecord
  validates :name, presence: true
  validates :email, allow_nil: true
  validates :phone, allow_blank: true
  validates :age, numericality: true
  validates :x, length: { is: 5 }, allow_nil: true, allow_blank: false
  validates :y, length: { is: 5 }, allow_blank: true
  validates :z, length: { is: 5 }, allow_nil: true
end

# Hash-rocket style should NOT be flagged (RuboCop checks key source text,
# which includes the leading `:` for rocket style, so key != 'allow_nil')
class WithRocketStyle
  validates :key_type, :inclusion => { :in => ['a', 'b'] }, :allow_blank => true, :allow_nil => true
  validates :fullname, :uniqueness => true, :allow_blank => true, :allow_nil => true
  validates :name, :allow_nil => true, :allow_blank => true, :format => { :with => /foo/ }
end
