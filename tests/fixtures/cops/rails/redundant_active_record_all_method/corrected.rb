User.where(active: true)
User.order(:name)
User.first
User.distinct
User.joins(:articles)
User.includes(:articles)
User.limit(10)
User.ids
User.pick(:id)
User.delete_all
User.destroy_all
user.articles.order(:created_at)
User.eager_load(:articles)
User.preload(:articles)
User.reorder(:created_at)
User.left_joins(:articles)
User.merge(users)
User.find_each(&:do_something)
User.take
User.second
User.sole
User.touch_all
User.update_all(name: name)
User.calculate(:average, :age)
User.delete_by(id: id)
User.destroy_by(id: id)
User.find_by!(name: name)
User.order(:created_at)
class Record < ApplicationRecord
  def self.active
    where(active: true)
  end
end
class Record < ::ApplicationRecord
  def self.recent
    order(:created_at)
  end
end
class Record < ActiveRecord::Base
  def self.names
    pluck(:name)
  end
end
class Record < ::ActiveRecord::Base
  def self.ids_list
    ids
  end
end
