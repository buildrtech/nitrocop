foo(1,
    2,
    3)
bar(:a,
    :b,
    :c)
baz("x",
    "y")
single_arg(1)

# Argument after closing brace of multiline hash (not first on its line)
enum :action, {
  none: 0,
  disable: 1_000,
}, suffix: :action

# Multiple arguments on one line after a multiline arg
contain_exactly(a_hash_including({
  name: 'bar',
}), a_hash_including({
  name: 'foo',
}))

# Bracket assignment []= is skipped by RuboCop
options['pre_chat_fields'][index] =
  field.deep_merge({
                     'label' => attribute['display_name'],
                     'placeholder' => attribute['display_name']
                   })

# Keyword args after **splat on same line — aligned with each other, not the splat
described_class.new(**default_attrs, index: 1,
                                     name: 'stash',
                                     branch: 'feature',
                                     message: 'WIP on feature')

# **splat followed by keyword args on continuation lines
redirect_to checkout_url(**params, host: DOMAIN, product: permalink,
                                   rent: item[:rental], recurrence: item[:recurrence],
                                   price: item[:price],
                                   code: code,
                                   affiliate_id: params[:id])

# **splat with two-space indented continuation
deprecate(**[:store, :update].index_with(MESSAGE),
  deprecator: ActiveResource.deprecator)

# Block arg &block aligned with first argument
tag.public_send tag_element,
                class: token_list(name, classes),
                data: { controller: "pagination" },
                **properties,
                &block
