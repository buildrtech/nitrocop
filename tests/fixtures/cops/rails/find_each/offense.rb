User.all.each { |u| u.save }
         ^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
User.where(active: true).each { |u| u.save }
                         ^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
User.includes(:posts).each { |u| u.save }
                      ^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
User.joins(:posts).each { |u| u.save }
                   ^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
User.all.each(&:save)
         ^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
User.where(active: true).each(&:activate!)
                         ^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
class Model < ApplicationRecord
  where(record: [record1, record2]).each(&:touch)
                                    ^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
end
class Model < ::ApplicationRecord
  all.each { |u| u.x }
      ^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
end
class Model < ActiveRecord::Base
  all.each { |u| u.x }
      ^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
end
class Model < ::ActiveRecord::Base
  all.each { |u| u.x }
      ^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
end
Host.all.each { |h| puts h }
         ^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
page.all(".selector").each { |q| q.x }
                      ^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
Poll.where(id: ids).each { |p| p.x }
                    ^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
Host.all.each do |host|
         ^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
  host.devices.where(type: "storage").order(:name).each { |d| d.process }
end
page.all(".question").each do |question|
                      ^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
  select "Single option", from: "Type"
end
Record.preload(:items).each do |record|
                       ^^^^ Rails/FindEach: Use `find_each` instead of `each` for batch processing.
  record.items.limit(10).pluck(:id)
end
