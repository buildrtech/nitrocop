!something

x = !foo

y = true

z = false

result = condition ? true : false

# not not is not flagged
not not something

# allowed_in_returns (default): !! at end of method body is OK
def active?
  !!@active
end

def valid?
  !!validate
end

def admin?
  !!current_user&.admin
end

# !! as part of a larger expression in return position
def comparison?
  !!simple_comparison(node) || nested_comparison?(node)
end

def allow_if_method_has_argument?(send_node)
  !!cop_config.fetch('AllowMethodsWithArguments', false) && send_node.arguments.any?
end

# !! with explicit return keyword
def foo?
  return !!bar if condition
  baz
  !!qux
end

# !! in if/elsif/else at return position
def foo?
  if condition
    !!foo
  elsif other
    !!bar
  else
    !!baz
  end
end

# !! in if/elsif/else with preceding statements at return position
def bar?
  if condition
    do_something
    !!foo
  elsif other
    do_something
    !!bar
  else
    do_something
    !!baz
  end
end

# !! in unless at return position
def foo?
  unless condition
    !!foo
  end
end

# !! in case/when at return position
def foo?
  case condition
  when :a
    !!foo
  when :b
    !!bar
  else
    !!baz
  end
end

# !! in rescue body at return position
def foo?
  bar
  !!baz.do_something
rescue
  qux
end

# !! in ensure body at return position
def foo?
  bar
  !!baz.do_something
ensure
  qux
end

# !! in rescue + ensure body at return position
def foo?
  bar
  !!baz.do_something
rescue
  qux
ensure
  corge
end

# !! in define_method block
define_method :foo? do
  bar
  !!qux
end

# !! in define_singleton_method block
define_singleton_method :foo? do
  bar
  !!qux
end

# !! with a line-broken expression at return position
def foo?
  return !!bar if condition
  baz
  !!qux &&
    quux
end

# !! in condition of if that is the last statement (line-based: on/after last stmt first line)
def snapshots_transporter?
  !!config.snapshots_transport_destination_url &&
  !!config.snapshots_transport_auth_key
end

# !! in XOR expression at last statement
def compare_metadata
  if (!!timekey ^ !!timekey2) || (!!tag ^ !!tag2)
    -1
  else
    0
  end
end

# !! as keyword argument value in method call at last statement
def start_server
  server_create(:in_tcp_server, @port, bind: @bind, resolve_name: !!@source_hostname_key) do |data|
    process(data)
  end
end

# !! in ternary at last statement
def render_response
  render json: json_obj, status: (!!success) ? 200 : 422
end

# !! as method argument at last statement
def validate_visibility(topic)
  !guardian.can_create_unlisted_topic?(topic, !!opts[:embed_url])
end

# !! in hash value as argument (keyword hash) at last statement
def fetch_topic_view
  render_json_dump(
    TopicViewPostsSerializer.new(
      @topic_view,
      scope: guardian,
      include_raw: !!params[:include_raw],
    ),
  )
end

# !! in array at last statement (array is method arg, not literal return)
def authenticate_with_http(username, password)
  result = user && authenticate(username, password)
  [!!result, username]
end

# !! in block body at last statement of method with tap
def page_layout_names(layoutpages: false)
  page_layout_definitions.select do |layout|
    !!layout.layoutpage && layoutpages || !layout.layoutpage && !layoutpages
  end.tap { _1.collect!(&:name) }
end

# !! on same line as last statement in if condition
def clear_capabilities(opts, target_file)
  if !!opts[:clear_capabilities]
    @capng.clear(:caps)
    ret = @capng.apply_caps_file(target_file)
  end
end

# !! in multi-line method call at last statement
def dynamic_class_creation?(node)
  !!node &&
    constant?(node) &&
    ["Class", "Module"].include?(constant_name(node))
end

# !! as part of multi-line boolean at last statement
def method_new_in_abstract_class?(attached_class, method_name)
  attached_class &&
    method_name == :new &&
    !!abstract_type_of(attached_class) &&
    Class === attached_class.singleton_class
end

# !! inside if condition at last statement of method
def is_block?(line)
  !!line.match(/^pattern_one/) \
  || !!line.match(/^pattern_two/)
end
