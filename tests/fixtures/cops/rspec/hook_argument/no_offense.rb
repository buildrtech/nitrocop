before { true }
after { true }
around { |example| example.run }
before(:suite) { true }
after(:context) { true }
before(:all) { setup_database }

# Explicit block-pass (`&handler`) is not an any_block hook and should be ignored.
state.before(:each, &handler)
state.after(:example, &handler)
