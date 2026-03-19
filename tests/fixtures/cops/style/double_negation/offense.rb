!!something
^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).

x = !!foo
    ^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).

!!nil
^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).

# !! not in the last position of a method body
def foo?
  foo
  !!test.something
  ^^^^^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
  bar
end

# !! inside hash values in return position (always an offense)
def foo
  { bar: !!baz, quux: value }
         ^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
end

# !! inside array values in return position (always an offense)
def foo
  [foo1, !!bar1, baz1]
         ^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
end

# !! inside multi-line hash in return position
def foo
  {
    bar: !!baz,
         ^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    quux: !!corge
          ^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
  }
end

# !! inside multi-line array in return position
def foo
  [
    foo1,
    !!bar1,
    ^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    !!baz1
    ^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
  ]
end

# !! not at return position inside unless
def foo?
  unless condition
    !!foo
    ^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    do_something
  end
end

# !! not at return position inside if/elsif/else
def foo?
  if condition
    !!foo
    ^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    do_something
  elsif other
    !!bar
    ^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    do_something
  else
    !!baz
    ^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    do_something
  end
end

# !! inside nested conditional where inner if ends before outer if/elsif
# RuboCop does NOT consider this return position because the inner conditional
# ends before the def body's last expression
def invite(username, invited_by, guardian)
  if condition_a
    if condition_b
      !!call_one(invited_by, guardian)
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    else
      !!call_two(invited_by, guardian)
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    end
  end
end

# !! in block body (not define_method) — not a return position
items.select do |item|
  !!item.active
  ^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
end

# !! in hash value in method call that is single-statement method body
# RuboCop digs into child_nodes.last of the call, finding the keyword hash
def augmented_section(title:, expanded: true, &block)
  render(
    partial: "/augmented/section",
    locals: { title:, expanded: !!expanded, block: }
                                ^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
  )
end

# !! in keyword args of method call as single-statement body
def create_migration
  FileStore.new(
    dry_run: !!ENV["DRY_RUN"],
             ^^^^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    migrate: !!ENV["MIGRATE"],
             ^^^^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
  )
end

# FN fix: !! as first element in multi-line || chain (single-statement body)
# parser_last_child digs into OrNode/AndNode, so last_child = right side on a later line
def has_interaction_matching?(request)
  !!matching_index_for(request) ||
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
  !!matching_used_for(request) ||
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
  @parent_list.has_matching?(request)
end

# FN fix: !! as first element in multi-line && chain (single-statement body)
def snapshots_transporter?
  !!config.snapshots_transport_destination_url &&
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
  !!config.snapshots_transport_auth_key
end

# FN fix: !! in multi-line && chain (single-statement body, not on last line)
def dynamic_class_creation?(node)
  !!node &&
  ^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    constant?(node) &&
    ["Class", "Module"].include?(constant_name(node))
end

# FN fix: !! in tap block call (single-statement body, block dig-in finds later last_child)
def page_layout_names(layoutpages: false)
  page_layout_definitions.select do |layout|
    !!layout.layoutpage && layoutpages || !layout.layoutpage && !layoutpages
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
  end.tap { _1.collect!(&:name) }
end

# FN fix: !! in hash value of ||= assignment (single-statement body)
# RuboCop digs into child_nodes.last of or_asgn, finding the hash
def devcontainer_options
  @devcontainer_options ||= {
    app_name: "myapp",
    database: !!defined?(ActiveRecord),
              ^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    active_storage: !!defined?(ActiveStorage),
                    ^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
  }
end

# !! in keyword arg of block call (single-statement body, block dig-in makes
# last_child = block body which is past the !! line)
def start_server
  server_create(:in_tcp_server, @port, bind: @bind, resolve_name: !!@source_hostname_key) do |data|
                                                                  ^^^^^^^^^^^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    process(data)
  end
end

# !! in hash value inside map block (block dig-in finds hash as last_child)
def run_actions
  items.map do |item|
    skipped = seen_items[item.name]
    { type: "recipe", name: item.name, skipped: !!skipped }
                                                ^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
  end
end
