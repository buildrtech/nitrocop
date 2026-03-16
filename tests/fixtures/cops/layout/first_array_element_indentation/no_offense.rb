x = [
  1,
  2,
  3
]

y = [1, 2, 3]

z = []

# special_inside_parentheses: array arg with [ on same line as (
foo([
      :bar,
      :baz
    ])

method_call(arg1, [
              :first,
              :second
            ])

expect(cli.run([
                 '--autocorrect-all',
                 '--only', 'Style/HashSyntax'
               ])).to eq(0)

create(:record, value: [
         { source_id: '1', inbox: inbox },
         { source_id: '2', inbox: inbox2 }
       ])

deeply.nested.call([
                     :a,
                     :b
                   ])

# Array with method chain uses line-relative indent
expect(x).to eq([
  'hello',
  'world'
].join("\n"))

# Array in grouping paren with operator uses line-relative indent
X = (%i[
  a
  b
] + other).freeze

# Array as RHS of % operator inside method call
gc.draw('text %d,%d %s' % [
  left.round + 2,
  header_height + 14,
  shell_quote(week_f.to_s)
])

# Indented % operator array in method body
  image.draw('rectangle %d,%d %d,%d' % [
    0, 0, width, height
  ])

# Array inside hash arg that is chained with .to_json (line-relative)
  client.should_receive(:api_post).
    with(endpoint, { requests: [
      { method: 'POST', url: 'v1.0/objects/Foo' }
    ], flag: true }.to_json).
    and_return(response)

# Another chained hash pattern
foo(status: 200, body: { responses: [
  { code: 200 },
  { code: 201 }
], total: 2 }.to_json)

# Array inside hash value with .compact chain (array is chained, not direct arg)
assert_equal({ "c" => [
  { "v" => 1421218800000, "f" => "Wed, Jan 14, 2015" },
  { "v" => 2, "f" => "2" },
].compact }, data["hits_over_time"]["rows"][1])

# Array in grouping parens with + operator and .shelljoin
command = (PREFIX + %W[
  convert
  #{image}
  -coalesce
]).shelljoin

# Array in grouping parens with + operator and .freeze
VALID_CONNECTION_KEYS = (VALID_REQUEST_KEYS + %i[
  ciphers
  client_key
  client_cert
]).freeze

# Array in grouping parens with - operator and .map
all_instances = (all_types - [
  PTypeReferenceType,
  PTypeAliasType
]).map { |c| c::DEFAULT }

# Array as hash value in multi-pair hash (no parens) - hash key relative indent
foo 1, bar: [
         2,
       ],
       baz: 3

# Array as hash value in multi-pair hash (nested)
[
  { subscription_line_items_attributes: [
      :id, :quantity, :variant_id, :price_estimate, :_destroy
    ],
    bill_address_attributes: Address.attributes,
    ship_address_attributes: Address.attributes }
]

# Array as hash value in multi-pair hash assignment
FILES = { ruby: [
            "app/**/*.rb",
            "lib/**/*.rake",
          ],
          js: [
            "app/assets/**/*.js",
          ] }

# Array as keyword arg value in method call (no parens)
acts_as_searchable columns: [
                     "#{table_name}.title",
                     "#{table_name}.notes"
                   ],
                   include: [:project],
                   date_column: "#{table_name}.created_at"

# Array with inner array chained with .join inside string interpolation
regex = [
  "[\"]([^\"]+)\"",
  "%(?:#{[
    '([\\W_])([^\\4]*)\\4',
    '\[([^\\]]*)\]',
  ].join('|')})"
].join('|')
