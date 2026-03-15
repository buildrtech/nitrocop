User.find_by_name("foo")
^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DynamicFindBy: Use `find_by(name: ...)` instead of `find_by_name`.
User.find_by_email("test@test.com")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DynamicFindBy: Use `find_by(email: ...)` instead of `find_by_email`.
Post.find_by_title("hello")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DynamicFindBy: Use `find_by(title: ...)` instead of `find_by_title`.
User.find_by_name!("foo")
^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DynamicFindBy: Use `find_by(name!: ...)` instead of `find_by_name!`.
User.find_by_name_and_email(name, email)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/DynamicFindBy: Use `find_by(name_and_email: ...)` instead of `find_by_name_and_email`.
class Account < ApplicationRecord
  def self.lookup(name)
    find_by_name(name)
    ^^^^^^^^^^^^^^^^^^ Rails/DynamicFindBy: Use `find_by(name: ...)` instead of `find_by_name`.
  end
end
class Record < ActiveRecord::Base
  def self.search(email)
    find_by_email(email)
    ^^^^^^^^^^^^^^^^^^^^ Rails/DynamicFindBy: Use `find_by(email: ...)` instead of `find_by_email`.
  end
end
