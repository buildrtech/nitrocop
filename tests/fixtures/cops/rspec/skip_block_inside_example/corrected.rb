it 'does something' do
  skip('TODO: reason')
end
specify 'another test' do
  skip('TODO: reason')
end
it 'third example' do
  skip('TODO: reason')
end
it "with skip nested deeper" do
  with_new_environment do
    RSpec.describe "some skipped test" do
      skip('TODO: reason')
    end
  end
end
it 'skip with block arg inside nested blocks' do
  database 'Customer' do
    table 'users' do
      skip('TODO: reason')
    end
  end
end
