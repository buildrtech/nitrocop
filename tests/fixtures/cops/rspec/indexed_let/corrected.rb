describe 'indexed lets' do
  let(:item) { create(:item) }
  let(:item) { create(:item) }
  let(:user) { create(:user) }
  let(:user) { create(:user) }
end

shared_examples 'indexed lets in shared group' do
  let(:record) { create(:record) }
  let(:record) { create(:record) }
end

shared_context 'indexed lets in shared context' do
  let(:entry) { create(:entry) }
  let(:entry) { create(:entry) }
end

shared_examples_for 'indexed lets in shared examples for' do
  let(:value) { 'a' }
  let(:value) { 'b' }
end

RSpec.shared_examples 'indexed lets with RSpec receiver' do
  let(:widget) { create(:widget) }
  let(:widget) { create(:widget) }
end

RSpec.shared_context 'indexed lets with RSpec.shared_context' do
  let(:payload) { build(:payload) }
  let(:payload) { build(:payload) }
end

context 'names with two numbers' do
  let(:user_1_item) { create(:item) }
  let(:user_1_item) { create(:item) }
  let(:user_2_item) { create(:item) }
end
