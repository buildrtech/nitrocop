it 'test', focus: true do
end
describe 'test', :focus do
end
context 'test', focus: true do
end
it 'test' do
end
describe 'test' do
end
::RSpec.describe 'test', :focus do
end
focus 'test' do
end
described_class.new(name:, focus:)
subject { described_class.new(command_runner:, focus:) }
Pane.new(commands: commands, focus: focus, layout: layout)
