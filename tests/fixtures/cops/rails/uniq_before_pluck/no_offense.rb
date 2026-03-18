Album.distinct.pluck(:band_name)
User.distinct.pluck(:email)
[1, 2, 2, 3].uniq
Album.pluck(:band_name)
Album.select(:band_name).distinct
# conservative mode: scope chain before pluck is not flagged
Model.scope.pluck(:name).uniq
# conservative mode: association-based (lowercase receiver) is not flagged
items.pluck(:name).uniq
# uniq before pluck is not flagged
Model.where(foo: 1).uniq.pluck(:something)
# uniq without a receiver
uniq.something
# uniq without pluck
Model.uniq
# uniq with a block
Model.where(foo: 1).pluck(:name).uniq { |k| k[0] }
# pluck without uniq receiver
pluck(:name).uniq
# uniq! is not flagged (RuboCop only restricts on :uniq, not :uniq!)
Album.pluck(:band_name).uniq!
Model.pluck(:name).uniq!
# Block body — RuboCop skips pluck.uniq inside block bodies (!^any_block)
cache { Model.pluck(:name).uniq }
get_set_cache("icons") { Badge.pluck(:icon).uniq }
items.map { |i| Item.pluck(:name).uniq }
# Multi-line block body (single statement)
cache do
  Model.pluck(:name).uniq
end
