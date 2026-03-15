expect(foo).to be(nil)
               ^^^^^^^ RSpec/BeNil: Prefer `be_nil` over `be(nil)`.
expect(bar).to be(nil)
               ^^^^^^^ RSpec/BeNil: Prefer `be_nil` over `be(nil)`.
expect(baz).not_to be(nil)
                   ^^^^^^^ RSpec/BeNil: Prefer `be_nil` over `be(nil)`.
doc.created_at.should be(nil)
                      ^^^^^^^ RSpec/BeNil: Prefer `be_nil` over `be(nil)`.
method.should_not be nil
                  ^^^^^^ RSpec/BeNil: Prefer `be_nil` over `be(nil)`.
its(:passive) { should be nil }
                       ^^^^^^ RSpec/BeNil: Prefer `be_nil` over `be(nil)`.
expect(stores).to all(be nil)
                      ^^^^^^ RSpec/BeNil: Prefer `be_nil` over `be(nil)`.
expect(subject.rc_file).to be_a(String).or be(nil)
                                           ^^^^^^^ RSpec/BeNil: Prefer `be_nil` over `be(nil)`.
