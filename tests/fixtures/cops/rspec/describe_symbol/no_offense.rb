describe("#some_method") { }

describe(MyClass) { }

describe MyClass, '#method' do
end

context(:some_method) { }

describe "a string" do
end

# Regular method calls named `describe` with an explicit receiver should not be flagged
expect(Statuses.describe(:foo)).to eq("something")
SomeClass.describe(:bar)
obj.describe(:baz)
