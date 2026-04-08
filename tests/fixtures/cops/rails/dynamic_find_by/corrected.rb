User.find_by(name: "foo")
User.find_by(email: "test@test.com")
Post.find_by(title: "hello")
User.find_by!(name: "foo")
User.find_by(name: name, email: email)
class Account < ApplicationRecord
  def self.lookup(name)
    find_by(name: name)
  end
end
class Record < ActiveRecord::Base
  def self.search(email)
    find_by(email: email)
  end
end
