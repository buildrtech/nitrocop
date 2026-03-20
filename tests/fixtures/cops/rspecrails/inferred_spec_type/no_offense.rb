# nitrocop-filename: spec/models/user_spec.rb
RSpec.describe User do
end

RSpec.describe User, type: :common do
end

RSpec.describe User, type: :controller do
end

# No described class — type: is the primary identifier, not redundant
RSpec.describe type: :model do
end

describe type: :model do
end
