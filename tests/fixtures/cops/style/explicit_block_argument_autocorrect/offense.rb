def with_block(&block)
  items.each { |x| yield x }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.

  super { yield }
  ^^^^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.

  items.each(1) { |x| yield x }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ExplicitBlockArgument: Consider using explicit block argument in the surrounding method's signature over `yield`.
end
