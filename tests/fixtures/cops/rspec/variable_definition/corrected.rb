RSpec.describe Foo do
  let(:user_name) { 'Adam' }
  let(:email) { 'test@example.com' }
  let!(:count) { 42 }
end

# Mail DSL subject with string arg inside an example group IS flagged
RSpec.describe Bar do
  it 'sends email' do
    Mail.new do
      subject :"testing message delivery"
    end
  end
end
