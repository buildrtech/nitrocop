foo(
  1
)
bar(
  2
)
baz(
  3
)

# super() with wrong indentation
super(
  serializer: Serializer,
        host: host,
        port: port.to_i
)

# Non-parenthesized call with backslash continuation — first arg on next line
output = Whenever.cron \
  <<-file
  set :job_template, nil
  every "weekday" do
    command "blahblah"
  end
file

# Another backslash continuation pattern
expect(subject.attributes).to eq \
  'alg' => 'test',
    'sub' => 'alice'

# Backslash continuation with wrong indent
assert_equal \
  "some long string value here",
  new_command.result.join(" ")

# Method call inside heredoc interpolation with wrong indentation
content = <<~HTML
  #{builder.attachment(
    :image,
      titled: true
  )}
HTML

# Tab-indented code with wrong indentation (3 tabs instead of expected 4)
		loader.inflector.inflect(
				"csv" => "CSV",
			"svg" => "SVG"
		)
