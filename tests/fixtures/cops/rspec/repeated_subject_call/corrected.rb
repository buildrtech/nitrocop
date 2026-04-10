RSpec.describe Foo do
  it do
    subject
    expect { skip('TODO: avoid repeated subject call') }.to not_change { Foo.count }
  end
end

RSpec.describe Bar do
  it do
    expect { subject }.to change { Bar.count }
    expect { skip('TODO: avoid repeated subject call') }.to not_change { Bar.count }
  end
end

RSpec.describe Baz do
  it do
    subject
    nested_block do
      expect { on_shard(:europe) { skip('TODO: avoid repeated subject call') } }.to not_change { Baz.count }
    end
  end
end

# Named subject alias
RSpec.describe Qux do
  subject(:bar) { do_something }

  it do
    bar
    expect { skip('TODO: avoid repeated subject call') }.to not_change { Qux.count }
  end
end

# Named subject used as constant path parent (mod::Params)
RSpec.describe TypeModule do
  subject(:mod) { Dry::Types.module }

  it "adds strict types as default" do
    expect(mod::Integer).to be(Dry::Types["integer"])
    expect(mod::Nominal::Integer).to be(Dry::Types["nominal.integer"])
    expect { skip('TODO: avoid repeated subject call')::Params }.to raise_error(NameError)
  end
end
