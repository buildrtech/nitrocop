let(:foo) do
  expect(something).to eq 'foo'
  ^^^^^^ RSpec/ExpectInLet: Do not use `expect` in let
end
let(:bar) do
  is_expected.to eq 'bar'
  ^^^^^^^^^^^ RSpec/ExpectInLet: Do not use `is_expected` in let
end
let(:baz) do
  expect_any_instance_of(Something).to receive :foo
  ^^^^^^^^^^^^^^^^^^^^^^ RSpec/ExpectInLet: Do not use `expect_any_instance_of` in let
end
let(:nested_block) do
  items.each { |i| expect(i).to be_valid }
                   ^^^^^^ RSpec/ExpectInLet: Do not use `expect` in let
end
let(:conditional) do
  if condition
    expect(value).to eq(1)
    ^^^^^^ RSpec/ExpectInLet: Do not use `expect` in let
  end
end
let(:ternary) do
  condition ? expect(value).to(eq(1)) : nil
              ^^^^^^ RSpec/ExpectInLet: Do not use `expect` in let
end
let(:logical_and) do
  valid && expect(result).to(be_truthy)
           ^^^^^^ RSpec/ExpectInLet: Do not use `expect` in let
end
let(:rescue_block) do
  begin
    expect(something).to eq(1)
    ^^^^^^ RSpec/ExpectInLet: Do not use `expect` in let
  rescue StandardError
    nil
  end
end
