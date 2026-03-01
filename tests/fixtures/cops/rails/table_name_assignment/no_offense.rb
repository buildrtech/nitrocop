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
