# Examples inside lambdas passed as arguments don't count as examples
# for the enclosing group. RuboCop can't statically detect them.
describe 'dynamic examples via lambda' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyExampleGroup: Empty example group detected.
  each_attribute -> (project, object, attrb) do
    next unless attrb.type == :simple
    it "#{attrb.name}=" do
      expect(true).to be(true)
    end
  end
end
