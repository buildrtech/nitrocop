User.all.find_each { |u| u.save }
User.where(active: true).find_each { |u| u.save }
User.includes(:posts).find_each { |u| u.save }
User.joins(:posts).find_each { |u| u.save }
User.all.find_each(&:save)
User.where(active: true).find_each(&:activate!)
class Model < ApplicationRecord
  where(record: [record1, record2]).find_each(&:touch)
end
class Model < ::ApplicationRecord
  all.find_each { |u| u.x }
end
class Model < ActiveRecord::Base
  all.find_each { |u| u.x }
end
class Model < ::ActiveRecord::Base
  all.find_each { |u| u.x }
end
Host.all.find_each { |h| puts h }
page.all(".selector").find_each { |q| q.x }
Poll.where(id: ids).find_each { |p| p.x }
Host.all.find_each do |host|
  host.devices.where(type: "storage").order(:name).each { |d| d.process }
end
page.all(".question").find_each do |question|
  select "Single option", from: "Type"
end
Record.preload(:items).find_each do |record|
  record.items.limit(10).pluck(:id)
end
