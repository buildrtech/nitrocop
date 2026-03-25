# nitrocop-filename: spec/models/user_spec.rb
RSpec.describe User, type: :model do
                     ^^^^^^^^^^^^ RSpecRails/InferredSpecType: Remove redundant spec type.
end

describe User, type: :model do
               ^^^^^^^^^^^^ RSpecRails/InferredSpecType: Remove redundant spec type.
end

RSpec.describe User, other: true, type: :model do
                                  ^^^^^^^^^^^^ RSpecRails/InferredSpecType: Remove redundant spec type.
end

xdescribe User, type: :model do
                ^^^^^^^^^^^^ RSpecRails/InferredSpecType: Remove redundant spec type.
end

# No described class with additional metadata — type: is NOT the only pair,
# so RuboCop's autocorrect doesn't crash and the offense is reported.
RSpec.describe type: :model, swars_spec: true do
               ^^^^^^^^^^^^ RSpecRails/InferredSpecType: Remove redundant spec type.
end
