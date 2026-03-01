class User < ActiveRecord::Base
  has_many :posts
end

class Post < ApplicationRecord
  belongs_to :user
end

# Base class exemption - STI base classes legitimately set table_name
class Base < ApplicationRecord
  self.table_name = 'special_table'
end

module Admin
  class Base < ApplicationRecord
    self.table_name = 'admin_records'
  end
end

# Interpolated strings are not flagged
class Widget < ApplicationRecord
  self.table_name = "#{table_name_prefix}widgets"
end

class Setting < ActiveRecord::Base
  self.table_name = "#{db_prefix}_settings"
end

# Not inside a class definition — included do block
module Concerns
  extend ActiveSupport::Concern
  included do
    self.table_name = "callback_modifiers"
  end
end

# Dynamic class with Class.new — not a class keyword
klass = Class.new(ApplicationRecord) do
  self.table_name = "good_jobs"
end

# Method call as RHS
class MessageClone < ApplicationRecord
  self.table_name = Message.table_name
end
