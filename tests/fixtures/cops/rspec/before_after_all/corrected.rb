before(:each) { do_something }
before(:each) { do_something }
after(:each) { do_something }
config.before(:each) { setup }
context.after(:each) { cleanup }
config.before :each do |group|
end
state.before(:each).each { |b| b.call }
expect(@state.before(:each)).to eq([@proc])
@shared.after(:each)
