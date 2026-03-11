include M
^^^^^^^^^ Style/MixinUsage: `include` is used at the top level. Use inside `class` or `module`.

extend N
^^^^^^^^ Style/MixinUsage: `extend` is used at the top level. Use inside `class` or `module`.

prepend O
^^^^^^^^^ Style/MixinUsage: `prepend` is used at the top level. Use inside `class` or `module`.

# Transparent wrappers: include inside def/if/begin at top level is still flagged
def foo
  include M
  ^^^^^^^^^ Style/MixinUsage: `include` is used at the top level. Use inside `class` or `module`.
end

if condition
  extend N
  ^^^^^^^^ Style/MixinUsage: `extend` is used at the top level. Use inside `class` or `module`.
end

begin
  prepend O
  ^^^^^^^^^ Style/MixinUsage: `prepend` is used at the top level. Use inside `class` or `module`.
end

include M1::M2::M3
^^^^^^^^^^^^^^^^^^ Style/MixinUsage: `include` is used at the top level. Use inside `class` or `module`.
