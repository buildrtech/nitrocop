# nitrocop-filename: spec/models/user_spec.rb
RSpec.describe User do
end

RSpec.describe User, type: :common do
end

RSpec.describe User, type: :controller do
end

# No described class, type mismatches inferred type — not redundant
RSpec.describe type: :controller do
end

describe type: :controller do
end

# No described class, type: is the only pair — RuboCop crashes on these
# (NoMethodError in autocorrect: left_sibling returns method name Symbol
# instead of AST node), so no offense is reported. Match that behavior.
RSpec.describe type: :model do
end

describe type: :model do
end
