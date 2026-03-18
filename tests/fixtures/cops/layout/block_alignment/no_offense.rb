items.each do |x|
  puts x
end

items.each { |x| puts x }

[1, 2].map do |x|
  x * 2
end

# end aligned with chain expression start (not the do-line)
@source_account.passive_relationships
               .where(account: Account.local)
               .in_batches do |follows|
  follows.update_all(target_account_id: 1)
end

# end aligned with call expression start in a hash value
def generate
  {
    data: items.map do |item|
            item.to_s
          end,
  }
end

# end aligned with call on previous line via backslash continuation
it 'does something' \
   'very interesting' do
  run_test
end

# end aligned with call on previous line via multiline args
option(opts, '--fail-level SEVERITY',
       RuboCop::Cop::Severity::NAMES) do |severity|
  @options[:fail_level] = severity
end

# end aligned with call expression that has multiline args ending with comma
add_offense(node,
            message: format(MSG,
                            flag: true)) do |corrector|
  corrector.replace(node, replacement)
end

# Multiline %i[] array with .each do block — end aligns with %i[
%i[opposite_style_detected unexpected_style_detected
   ambiguous_style_detected conflicting_styles_detected
   unrecognized_style_detected
   no_acceptable_style!].each do |method|
  puts method
end

# do...end block inside a brace block — end aligns with chain root
to = lambda { |env|
  hostess.call(env)
    .tap do |response|
      response[1].delete("x-cascade")
  end
}

# string concatenation with + on multiline method description (RSpec-style)
it "should convert " +
    "some value " +
    "correctly" do
  run_test
end

# string concatenation with + on multiline describe block
describe User, "when created with a name known to cause issues " +
    "in certain cases" do
  it "should work" do
    expect(true).to be true
  end
end

# block as argument inside parentheses — end aligns with method inside parens
expect(arr.all? do |o|
         o.valid?
       end)

# multiline with || continuation: do on second line
a ||
  items.each do |x|
  process(x)
end

# multiline with && continuation: do on second line
(value.is_a? Array) &&
  value.all? do |subvalue|
  type_check(subvalue)
end

# FP fix: multiline string literal (no explicit continuation) -- do on wrapped line
it "returns remote census api response when available and valid without send
    date_of_birth and postal_code" do
  response = api.call(1, "A", nil, nil)
  expect(response).to eq(true)
end

# FP fix: multiline args with comments between continuation lines
it "navigates correctly the path from the overview page to boards",
   # The polling interval is lowered for testing
   # In reality, it does not really matter
   with_settings: { notifications_polling_interval: 1_000 } do
  visit project_path(project)
  expect(true).to eq(true)
end

# FP fix: multiline string with backslash continuation spanning many lines
describe "WHEN the user is allowed to update entries " \
         "WHEN updating:
            entity_type
            entity_id
            user_id
            units
            cost_type
            spent_on" do
  let(:expected_entity) { build(:entity) }
end

# FP fix: describe block with multiline arg and comma before do
add_api_endpoint "API::V3::Users::UsersAPI", :id,
                 caption: ->(*) { I18n.t("label") },
                 if: ->(*) { enabled? },
                 icon: "image" do
  mount ::API::V3::Users::UserAvatarAPI
end

# FP fix: chained block end aligns with method name in assignment context
response = stub_comms do
             verify_something
           end.check_request do |data|
  assert_match(/pattern/, data)
end.respond_with(response)

# FP fix: end&.path aligns with method name in assignment context
tmpl_path = caller_locations(1, 2).find do |loc|
              loc.label.include?("method_missing").!
            end&.path

# FP fix: && on same line as do — end aligns with LHS of && expression
next true if urls&.size&.positive? && urls&.all? do |url|
               url.include?(T.must(cred["registry"]))
             end


# Lambda/proc brace block } aligns with -> start or line indent
scope :last_n_per_feed, -> (n, feed_ids) {
   ranked_posts = select(select_sql)
   from(ranked_posts, "entries")
     .where("entries_rank <= ?", n)
     .where(feed_id: feed_ids)
}

# Lambda brace block } aligns with -> start or line indent
[:favorite_even_number, validate_with: -> (v) {
   unless v.nil? || v.even?
     {code: :even, msg: "Value must be even. Was: #{v}"}
   end
}]
