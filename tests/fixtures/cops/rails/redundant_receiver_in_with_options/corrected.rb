class Account < ApplicationRecord
  with_options dependent: :destroy do |assoc|
    has_many :customers
    has_many :products
    has_many :invoices
  end
end
