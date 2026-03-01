it do
  foo = instance_double(Foo).as_null_object
        ^^^^^^^^^^^^^^^^^^^^ RSpec/InstanceSpy: Use `instance_spy` when you check your double with `have_received`.
  expect(foo).to have_received(:something)
end

it do
  bar = instance_double(Bar).as_null_object
        ^^^^^^^^^^^^^^^^^^^^ RSpec/InstanceSpy: Use `instance_spy` when you check your double with `have_received`.
  expect(bar).to have_received(:something)
end

it do
  baz = instance_double(Baz, :name).as_null_object
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/InstanceSpy: Use `instance_spy` when you check your double with `have_received`.
  expect(baz).to have_received(:something)
end
