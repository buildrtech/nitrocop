describe 'indexed lets' do
  let(:item_1) { create(:item) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `1` in its name. Please give it a meaningful name.
  let(:item_2) { create(:item) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `2` in its name. Please give it a meaningful name.
  let(:user1) { create(:user) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `1` in its name. Please give it a meaningful name.
  let(:user2) { create(:user) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `2` in its name. Please give it a meaningful name.
end

shared_examples 'indexed lets in shared group' do
  let(:record_1) { create(:record) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `1` in its name. Please give it a meaningful name.
  let(:record_2) { create(:record) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `2` in its name. Please give it a meaningful name.
end

shared_context 'indexed lets in shared context' do
  let(:entry_1) { create(:entry) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `1` in its name. Please give it a meaningful name.
  let(:entry_2) { create(:entry) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `2` in its name. Please give it a meaningful name.
end

shared_examples_for 'indexed lets in shared examples for' do
  let(:value_1) { 'a' }
  ^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `1` in its name. Please give it a meaningful name.
  let(:value_2) { 'b' }
  ^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `2` in its name. Please give it a meaningful name.
end

RSpec.shared_examples 'indexed lets with RSpec receiver' do
  let(:widget_1) { create(:widget) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `1` in its name. Please give it a meaningful name.
  let(:widget_2) { create(:widget) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `2` in its name. Please give it a meaningful name.
end

RSpec.shared_context 'indexed lets with RSpec.shared_context' do
  let(:payload_1) { build(:payload) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `1` in its name. Please give it a meaningful name.
  let(:payload_2) { build(:payload) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `2` in its name. Please give it a meaningful name.
end

context 'names with two numbers' do
  let(:user_1_item_1) { create(:item) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `1` in its name. Please give it a meaningful name.
  let(:user_1_item_2) { create(:item) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `1` in its name. Please give it a meaningful name.
  let(:user_2_item_1) { create(:item) }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/IndexedLet: This `let` statement uses `2` in its name. Please give it a meaningful name.
end
