class User < ApplicationRecord
  has_many :posts
  has_many :posts, foreign_key: :author_id, inverse_of: :author
  has_one :profile, as: :profilable, inverse_of: :user
  belongs_to :company
  has_many :followers, -> { order(:name) }, through: :relationships
  belongs_to :imageable, polymorphic: true
  has_many :active_accounts, -> { merge(Account.active) }, through: :memberships, source: :account
end

# Associations inside included blocks with inverse_of set
module Concern
  extend ActiveSupport::Concern

  included do
    has_many :editions, foreign_key: :edition_id, inverse_of: :document
    has_many :versions, -> { order(:created_at) }, inverse_of: :versionable
    has_many :items, through: :memberships
  end
end

# lambda scope with inverse_of set
class Person < ApplicationRecord
  has_many :role_appointments,
           lambda { order(:ordering) },
           inverse_of: :person
end

# Dynamic options (double splat) -- should not flag because **options may contain inverse_of
class Item < ApplicationRecord
  def define_association(**options)
    has_many :entries, conditions: -> { where(active: true) }, **options
  end
end

# with_options providing inverse_of
class Article < ApplicationRecord
  with_options inverse_of: false do
    has_one :header, -> { where(active: true) }
  end
end

# foreign_key: nil does not require inverse_of
class Record < ApplicationRecord
  has_many :items, foreign_key: nil
end

# with_options providing through: should suppress offense
class Poll < ApplicationRecord
  with_options class_name: 'Account', source: :account, through: :votes do
    has_many :voters, -> { group('accounts.id') }
    has_many :local_voters, -> { group('accounts.id').merge(Account.local) }
  end
end
