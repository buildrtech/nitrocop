hash.merge!(a: 1)
^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
hash.merge!(key: value)
^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
opts.merge!(debug: true)
^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
h = {}
h.merge!(a: 1, b: 2)
^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with 2 key-value pairs.
puts "done"
settings = {}
settings.merge!(Port: port, Host: bind)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with 2 key-value pairs.
start_server
jar = cookies('foo=bar')
jar.merge! :bar => 'baz'
^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
expect(jar).to include('bar')
# instance variable receiver — pure, should be flagged
@params.merge!(a: 1)
^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
# class variable receiver — pure, should be flagged
@@defaults.merge!(key: value)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
# constant receiver — pure, should be flagged
DEFAULTS.merge!(key: value)
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
# ivar receiver with multiple pairs
@params.merge!(a: 1, b: 2)
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with 2 key-value pairs.
# self receiver — pure, should be flagged
self.merge!(key: value)
^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
# merge! on accumulator inside each_with_object — value not truly used
ENUM.each_with_object({}) do |e, h|
  h.merge!(e => e)
  ^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
end
items.each_with_object({}) { |style, memo| memo.merge!(style["name"] => style["value"]) }
                                           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
config.each_with_object({}) { |key, filter| filter.merge!(key => []) }
                                            ^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
# hash rocket inside do..while inside begin/rescue
def list_files
  begin
    begin
      response = client.list_objects(options)
      break if response[:contents].empty?
      s3_options.merge!(:marker => response[:contents].last[:key])
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
    end while response[:truncated]
  rescue Errno::EPIPE
    nil
  end
end
