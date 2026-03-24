class User < ApplicationRecord
  has_many :posts
  ^^^^^^^^^^^^^^ Rails/DuplicateAssociation: Association `posts` is defined multiple times. Don't repeat associations.
  has_many :posts, dependent: :destroy
  ^^^^^^^^ Rails/DuplicateAssociation: Association `posts` is defined multiple times. Don't repeat associations.
end

class Post < ApplicationRecord
  belongs_to :author
  ^^^^^^^^^^^^^^^^^^ Rails/DuplicateAssociation: Association `author` is defined multiple times. Don't repeat associations.
  belongs_to :author, optional: true
  ^^^^^^^^^^ Rails/DuplicateAssociation: Association `author` is defined multiple times. Don't repeat associations.
end

class Company < ApplicationRecord
  has_one :address
  ^^^^^^^^^^^^^^^^ Rails/DuplicateAssociation: Association `address` is defined multiple times. Don't repeat associations.
  has_one :address, dependent: :destroy
  ^^^^^^^ Rails/DuplicateAssociation: Association `address` is defined multiple times. Don't repeat associations.
end

# has_and_belongs_to_many duplicates
class Tag < ApplicationRecord
  has_and_belongs_to_many :articles
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DuplicateAssociation: Association `articles` is defined multiple times. Don't repeat associations.
  has_and_belongs_to_many :articles
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DuplicateAssociation: Association `articles` is defined multiple times. Don't repeat associations.
end

# String argument instead of symbol
class Invoice < ApplicationRecord
  has_many 'items'
  ^^^^^^^^^^^^^^^^ Rails/DuplicateAssociation: Association `items` is defined multiple times. Don't repeat associations.
  has_many 'items', dependent: :destroy
  ^^^^^^^^ Rails/DuplicateAssociation: Association `items` is defined multiple times. Don't repeat associations.
end

# class_name duplicate detection (has_many, not belongs_to)
class Account < ApplicationRecord
  has_many :foos, class_name: 'Foo'
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DuplicateAssociation: Association `class_name: 'Foo'` is defined multiple times. Don't repeat associations.
  has_many :bars, class_name: 'Foo'
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DuplicateAssociation: Association `class_name: 'Foo'` is defined multiple times. Don't repeat associations.
end

# class_name duplicate detection (has_one)
class Profile < ApplicationRecord
  has_one :baz, class_name: 'Bar'
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DuplicateAssociation: Association `class_name: 'Bar'` is defined multiple times. Don't repeat associations.
  has_one :qux, class_name: 'Bar'
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DuplicateAssociation: Association `class_name: 'Bar'` is defined multiple times. Don't repeat associations.
end

# belongs_to in if/else branches (lorint/brick pattern)
class IfBranchModel < ActiveRecord::Base
  if ActiveRecord.version >= Gem::Version.new('5.0')
    belongs_to :person, optional: true
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DuplicateAssociation: Association `person` is defined multiple times. Don't repeat associations.
  else
    belongs_to :person
    ^^^^^^^^^^^^^^^^^^ Rails/DuplicateAssociation: Association `person` is defined multiple times. Don't repeat associations.
  end
end
