class User < ApplicationRecord
  validates :x, length: { is: 5 }, allow_blank: true
end

class Post < ApplicationRecord
  validates :x, length: { is: 5 }, allow_blank: false
end

class Comment < ApplicationRecord
  validates :x, length: { is: 5 }, allow_blank: true
end

# allow_nil and allow_blank nested inside a validator option hash
class NestingSpec
  validates :ie_condition, inclusion: { in: ['a', 'b'], allow_blank: true }
  validates :role, inclusion: { in: ['x', 'y'], allow_blank: true }
end
