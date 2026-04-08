class User < ApplicationRecord
  attribute :tags, default: -> { [] }
end

class Post < ApplicationRecord
  attribute :metadata, default: -> { {} }
end

class Order < ApplicationRecord
  attribute :confirmed_at, :datetime, default: -> { Time.zone.now }
end

class Item < ApplicationRecord
  attribute :id, default: -> { proc(&::Kind::ID) }
  attribute :owner_id, default: -> { proc(&::Kind::ID) }
  attribute :description, default: -> { proc(&::Todo::Description) }
end
