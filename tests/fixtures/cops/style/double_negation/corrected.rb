!something.nil?

x = !foo.nil?

!nil.nil?

# !! not in the last position of a method body
def foo?
  foo
  !test.something.nil?
  bar
end

# !! inside hash values in return position (always an offense)
def foo
  { bar: !baz.nil?, quux: value }
end

# !! inside array values in return position (always an offense)
def foo
  [foo1, !bar1.nil?, baz1]
end

# !! inside multi-line hash in return position
def foo
  {
    bar: !baz.nil?,
    quux: !corge.nil?
  }
end

# !! inside multi-line array in return position
def foo
  [
    foo1,
    !bar1.nil?,
    !baz1.nil?
  ]
end

# !! not at return position inside unless
def foo?
  unless condition
    !foo.nil?
    do_something
  end
end

# !! not at return position inside if/elsif/else
def foo?
  if condition
    !foo.nil?
    do_something
  elsif other
    !bar.nil?
    do_something
  else
    !baz.nil?
    do_something
  end
end

# !! inside nested conditional where inner if ends before outer if
# Multi-statement body: outer if is NOT the only statement, so child_nodes.last
# of begin = outer if. Inner conditional ends before outer if's end → offense.
def invite(username, invited_by, guardian)
  do_setup
  if condition_a
    if condition_b
      !call_one(invited_by, guardian).nil?
    else
      !call_two(invited_by, guardian).nil?
    end
  end
end

# !! inside elsif branch in multi-statement body (confirmed FN)
# When def body has multiple statements, last_child = outer if. The elsif
# conditional ends before the outer if's end → not return position → offense.
def circular_reference?(child_svc)
  return true if child_svc == self

  if child_svc.kind_of?(Service)
    ancestor_ids.include?(child_svc.id)
  elsif child_svc.kind_of?(ServiceTemplate)
    !circular_reference_check(child_svc).nil?
  end
end

# !! with =~ inside elsif branch in multi-statement body (confirmed FN)
def is_a_type?(value)
  value = compute_type(value)

  if value == :system_snapshot
    is_a_type?(:evm_snapshot)
  elsif value.kind_of?(Regexp)
    !(value =~ name).nil?
  else
    name == value
  end
end

# !! in block body (not define_method) — not a return position
items.select do |item|
  !item.active.nil?
end

# !! in hash value in method call that is single-statement method body
# RuboCop digs into child_nodes.last of the call, finding the keyword hash
def augmented_section(title:, expanded: true, &block)
  render(
    partial: "/augmented/section",
    locals: { title:, expanded: !expanded.nil?, block: }
  )
end

# !! in keyword args of method call as single-statement body
def create_migration
  FileStore.new(
    dry_run: !ENV["DRY_RUN"].nil?,
    migrate: !ENV["MIGRATE"].nil?,
  )
end

# FN fix: !! as first element in multi-line || chain (single-statement body)
# parser_last_child digs into OrNode/AndNode, so last_child = right side on a later line
def has_interaction_matching?(request)
  !matching_index_for(request).nil? ||
  !matching_used_for(request).nil? ||
  @parent_list.has_matching?(request)
end

# FN fix: !! as first element in multi-line && chain (single-statement body)
def snapshots_transporter?
  !config.snapshots_transport_destination_url.nil? &&
  !!config.snapshots_transport_auth_key
end

# FN fix: !! in multi-line && chain (single-statement body, not on last line)
def dynamic_class_creation?(node)
  !node.nil? &&
    constant?(node) &&
    ["Class", "Module"].include?(constant_name(node))
end

# FN fix: !! in tap block call (single-statement body, block dig-in finds later last_child)
def page_layout_names(layoutpages: false)
  page_layout_definitions.select do |layout|
    !layout.layoutpage.nil? && layoutpages || !layout.layoutpage && !layoutpages
  end.tap { _1.collect!(&:name) }
end

# FN fix: !! in hash value of ||= assignment (single-statement body)
# RuboCop digs into child_nodes.last of or_asgn, finding the hash
def devcontainer_options
  @devcontainer_options ||= {
    app_name: "myapp",
    database: !defined?(ActiveRecord).nil?,
    active_storage: !defined?(ActiveStorage).nil?,
  }
end

# !! in keyword arg of block call (single-statement body, block dig-in makes
# last_child = block body which is past the !! line)
def start_server
  server_create(:in_tcp_server, @port, bind: @bind, resolve_name: !@source_hostname_key.nil?) do |data|
    process(data)
  end
end

# FN fix (round 8): !! inside parens used in ternary condition inside hash value
# `(!!expr && other)` — the `!!` node's non-enumerable parent is `and`, NOT
# `begin`/parens, so the lenient same-line check should NOT apply.
# parenthesized_last_line must not leak through and/or nodes.
def self.from_hash(hash)
  page = self.new(URI(hash['url']))
  {
    '@visited' => hash['visited'],
    '@redirect_to' => (!hash['redirect_to'].nil? && !hash['redirect_to'].empty?) ? URI(hash['redirect_to']) : nil,
    '@fetched' => hash['fetched']
  }.each do |var, value|
    page.instance_variable_set(var, value)
  end
  page
end

# FN fix (round 8): !! inside parens in || condition within nested blocks
# `(_options && !!_options[:allow_nil])` — the non-enumerable parent of `!!`
# is `and`, not `begin`, so strict check applies: offense.
def as_json(methods = nil)
  methods ||= self.getters
  {}.tap do |hash|
    methods.each do |meth|
      _options = self.property_options[meth]
      if _options[:if].blank? || self.instance_eval(&_options[:if])
        value = self.send(meth.to_sym)
        if !value.nil? || (_options && !_options[:allow_nil].nil?)
          hash[meth] = value
        end
      end
    end
  end
end

