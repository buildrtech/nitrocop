it 'test', focus: true do
           ^^^^^^^^^^^ RSpec/Focus: Focused spec found.
end
describe 'test', :focus do
                 ^^^^^^ RSpec/Focus: Focused spec found.
end
context 'test', focus: true do
                ^^^^^^^^^^^ RSpec/Focus: Focused spec found.
end
fit 'test' do
^^^^^^^^^^ RSpec/Focus: Focused spec found.
end
fdescribe 'test' do
^^^^^^^^^^^^^^^^ RSpec/Focus: Focused spec found.
end
::RSpec.describe 'test', :focus do
                         ^^^^^^ RSpec/Focus: Focused spec found.
end
focus 'test' do
^^^^^^^^^^^^ RSpec/Focus: Focused spec found.
end
described_class.new(name:, focus:)
                           ^^^^^ RSpec/Focus: Focused spec found.
subject { described_class.new(command_runner:, focus:) }
                                               ^^^^^ RSpec/Focus: Focused spec found.
Pane.new(commands: commands, focus: focus, layout: layout)
                                    ^^^^^ RSpec/Focus: Focused spec found.
