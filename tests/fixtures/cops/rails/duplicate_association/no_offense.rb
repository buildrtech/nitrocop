class User < ApplicationRecord
  has_many :posts, dependent: :destroy
  has_one :profile, dependent: :destroy
  belongs_to :company
end

# Serializer classes should not be flagged
class CollectionSerializer < ActivityPub::Serializer
  has_many :items, key: :items, if: -> { condition_a }
  has_many :items, key: :ordered_items, if: -> { condition_b }
end

# has_and_belongs_to_many without duplicates
class Category < ApplicationRecord
  has_and_belongs_to_many :posts
  has_and_belongs_to_many :tags
end

# belongs_to with same class_name is NOT flagged
class Order < ApplicationRecord
  belongs_to :foos, class_name: 'Foo'
  belongs_to :bars, class_name: 'Foo'
end

# class_name with extra options is NOT flagged
class Report < ApplicationRecord
  has_many :foos, if: :condition, class_name: 'Foo'
  has_many :bars, if: :some_condition, class_name: 'Foo'
  has_one :baz, -> { condition }, class_name: 'Bar'
  has_one :qux, -> { some_condition }, class_name: 'Bar'
end

# Associations with do...end blocks are excluded from duplicate checking
# (RuboCop's each_child_node(:send) skips block-wrapped calls)
class Author < ApplicationRecord
  has_many :posts_containing_the_letter_a, :class_name => "Post"
  has_many :posts_with_extension, :class_name => "Post" do
    def testing
      true
    end
  end
end

# Name duplicate with block should also not be flagged
class Publisher < ApplicationRecord
  has_many :books
  has_many :books do
    def bestsellers
      where(bestseller: true)
    end
  end
end
