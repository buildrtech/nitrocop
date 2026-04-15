obj.send(:method_name)
obj.respond_to?(:foo)
obj.method(:bar)
obj.public_send(:baz)
obj.define_method(:my_method) { }
obj.instance_variable_get(:@ivar)
# Command methods (receiverless)
attr_accessor :name, :role
alias_method :new_name, :old_name
private :helper
# Hyphenated strings are valid symbols (:'payment-sources')
doc.send(:"payment-sources") { }
# Empty strings are valid symbols (:""")
obj.send(:"")
# Null byte strings are valid symbols (:"\x00")
obj.send(:"\x00")
