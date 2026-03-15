class User < ApplicationRecord
  validates :x, length: { is: 5 }, allow_nil: true, allow_blank: true
                                   ^^^^^^^^^^^^^^^ Rails/RedundantAllowNil: `allow_nil` is redundant when `allow_blank` has the same value.
end

class Post < ApplicationRecord
  validates :x, length: { is: 5 }, allow_nil: false, allow_blank: false
                                   ^^^^^^^^^^^^^^^^ Rails/RedundantAllowNil: `allow_nil` is redundant when `allow_blank` has the same value.
end

class Comment < ApplicationRecord
  validates :x, length: { is: 5 }, allow_nil: false, allow_blank: true
                                   ^^^^^^^^^^^^^^^^ Rails/RedundantAllowNil: `allow_nil: false` is redundant when `allow_blank` is true.
end

# allow_nil and allow_blank nested inside a validator option hash
class NestingSpec
  validates :ie_condition, inclusion: { in: ['a', 'b'], allow_nil: true, allow_blank: true }
                                                        ^^^^^^^^^^^^^^^ Rails/RedundantAllowNil: `allow_nil` is redundant when `allow_blank` has the same value.
  validates :role, inclusion: { in: ['x', 'y'], allow_blank: true, allow_nil: true }
                                                                   ^^^^^^^^^^^^^^^ Rails/RedundantAllowNil: `allow_nil` is redundant when `allow_blank` has the same value.
end
