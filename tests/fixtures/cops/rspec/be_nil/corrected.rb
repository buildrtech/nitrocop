expect(foo).to be_nil
expect(bar).to be_nil
expect(baz).not_to be_nil
doc.created_at.should be_nil
method.should_not be_nil
its(:passive) { should be_nil }
expect(stores).to all(be_nil)
expect(subject.rc_file).to be_a(String).or be_nil
