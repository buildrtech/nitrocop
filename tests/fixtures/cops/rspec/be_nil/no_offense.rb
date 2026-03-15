expect(foo).to be_nil
expect(foo).to be(true)
expect(foo).to be(false)
expect(foo).to be(1)
expect(foo).to be_truthy
doc.created_at.should be_nil
method.should_not be_truthy
its(:passive) { should be_nil }
expect(stores).to all(be_nil)
expect(subject.rc_file).to be_a(String).or be_nil
