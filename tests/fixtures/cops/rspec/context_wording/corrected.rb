context 'when the display name not present' do
end

context 'when whenever you do' do
end

shared_context 'when the display name not present' do
end

# Interpolated string descriptions should also be checked
context "when Fabricate(:#{fabricator_name})" do
end

# Backtick (xstr) descriptions should also be checked
context `when the display name not present` do
end

context `when bad #{interpolated} description` do
end

# Interpolated string starting with interpolation (no leading text)
context "when #{var_name} elements" do
end

# Interpolated string that is purely interpolation
context "when #{description}" do
end
