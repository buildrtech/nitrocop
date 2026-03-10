def foo(&block)
        ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  bar(&block)
      ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
def baz(&block)
        ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  yield_with(&block)
             ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
def qux(&block)
        ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  other(&block)
        ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
# yield is forwarding
def with_yield(&block)
               ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  yield
end
# unused block param (no body)
def empty_body(&block)
               ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
# unused block param (body exists but doesn't reference block)
def unused_param(&block)
                 ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  something_else
end
# symbol proc in body (block unused)
def with_symbol_proc(&block)
                     ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  bar(&:do_something)
end
# forwarding in singleton method
def self.singleton_fwd(&block)
                       ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  bar(&block)
      ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
# multiple forwarding usages
def multi_forward(&block)
                  ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  bar(&block)
      ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  baz(qux, &block)
           ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
# forwarding without parentheses
def no_parens arg, &block
                   ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  bar &block
      ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  baz qux, &block
           ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
# forwarding with other proc arguments
def other_procs(bar, &block)
                     ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  delegator.foo(&bar).each(&block)
                           ^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
# space between & and param name — body with no space is NOT flagged (source mismatch)
def transmit uri, req, payload, & block
                                ^^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  process_result(res, start_time, tempfile, &block)
end
# space between & and param name — body also has space, so sources MATCH → both flagged
def forward_with_space(& blk)
                       ^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  other_method(& blk)
               ^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
# space in both param and multiple body usages
def method_missing(s, *args, & b)
                             ^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  r.send(s, *args, & b)
                   ^^^ Naming/BlockForwarding: Use anonymous block forwarding.
end
# space in param, mixed body usages: one matches, one doesn't
def mixed_spaces(& block)
                 ^^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  foo(& block)
      ^^^^^^^ Naming/BlockForwarding: Use anonymous block forwarding.
  bar(&block)
end
