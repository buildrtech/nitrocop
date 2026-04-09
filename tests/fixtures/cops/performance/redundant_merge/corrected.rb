hash[:a] = 1
hash[:key] = value
opts[:debug] = true
h = {}
h.merge!(a: 1, b: 2)
puts "done"
settings = {}
settings.merge!(Port: port, Host: bind)
start_server
jar = cookies('foo=bar')
jar[:bar] = 'baz'
expect(jar).to include('bar')
# instance variable receiver — pure, should be flagged
@params[:a] = 1
# class variable receiver — pure, should be flagged
@@defaults[:key] = value
# constant receiver — pure, should be flagged
DEFAULTS[:key] = value
# ivar receiver with multiple pairs
@params.merge!(a: 1, b: 2)
# self receiver — pure, should be flagged
self[:key] = value
# merge! on accumulator inside each_with_object — value not truly used
ENUM.each_with_object({}) do |e, h|
  h[e] = e
end
items.each_with_object({}) { |style, memo| memo[style["name"]] = style["value"] }
config.each_with_object({}) { |key, filter| filter[key] = [] }
# hash rocket inside do..while inside begin/rescue
def list_files
  begin
    begin
      response = client.list_objects(options)
      break if response[:contents].empty?
      s3_options[:marker] = response[:contents].last[:key]
    end while response[:truncated]
  rescue Errno::EPIPE
    nil
  end
end
