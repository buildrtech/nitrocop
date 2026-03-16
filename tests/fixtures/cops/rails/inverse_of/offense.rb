class User < ApplicationRecord
  has_many :posts, foreign_key: :author_id
  ^^^^^^^^ Rails/InverseOf: Specify an `:inverse_of` option.
  belongs_to :company, foreign_key: :org_id
  ^^^^^^^^^^ Rails/InverseOf: Specify an `:inverse_of` option.
  has_one :avatar, foreign_key: :owner_id
  ^^^^^^^ Rails/InverseOf: Specify an `:inverse_of` option.
end

# Associations inside included blocks (concerns)
module Edition::ActiveEditors
  extend ActiveSupport::Concern

  included do
    has_many :recent_edition_openings, foreign_key: :edition_id, dependent: :destroy
    ^^^^^^^^ Rails/InverseOf: Specify an `:inverse_of` option.
  end
end

# Associations with lambda {} scope (not -> {})
class Person < ApplicationRecord
  has_many :role_appointments,
  ^^^^^^^^ Rails/InverseOf: Specify an `:inverse_of` option.
           lambda {
             order(:ordering)
           }
end

# Associations with scope and as: option inside included block
module Concern
  extend ActiveSupport::Concern

  included do
    has_many :versions, -> { order(created_at: :asc) }, as: :item
    ^^^^^^^^ Rails/InverseOf: Specify an `:inverse_of` option.
  end
end

# inverse_of: nil should flag with a different message
class Account < ApplicationRecord
  has_many :entries, -> { order(:date) }, inverse_of: nil
  ^^^^^^^^ Rails/InverseOf: You specified `inverse_of: nil`, you probably meant to use `inverse_of: false`.
end

# Association calls with explicit receiver (e.g., base.has_many in concerns)
module Reactable
  def self.included(base)
    base.has_many :reactions, -> { joins(:user).where("users.deactivated_at": nil) }, dependent: :destroy, as: :reactable
         ^^^^^^^^ Rails/InverseOf: Specify an `:inverse_of` option.
  end
end
