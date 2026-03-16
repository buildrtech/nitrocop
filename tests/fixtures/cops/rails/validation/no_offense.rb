class User < ApplicationRecord
  validates :name, presence: true
  validates :email, uniqueness: true
  validates :age, numericality: { greater_than: 0 }
  validates :role, inclusion: { in: %w[admin user] }
end

# No arguments — RuboCop skips these
validates_numericality_of
validates_presence_of

# Last argument is a non-literal (send/variable/constant) — RuboCop skips these
validates_numericality_of :a, b
validates_numericality_of :a, B
b = { minimum: 1 }
validates_numericality_of :a, b

# Single argument is a local variable (from block parameter)
items.each do |field|
  validates_presence_of field
end
