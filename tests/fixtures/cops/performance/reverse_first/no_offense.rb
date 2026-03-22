[1, 2, 3].last
[1, 2, 3].last(2)
[1, 2, 3].first
arr.reverse
arr.first
items.reverse.first(count)
items.reverse.first(MAX_COUNT)
items.reverse.first(get_count)

# .reverse with arguments is Sequel's ordering method, not Array#reverse
self.filter(col => id).reverse(:retired_at_epoch_ms).first
items.order(:name).reverse(:created_at).first
