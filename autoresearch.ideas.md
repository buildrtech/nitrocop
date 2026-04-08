# Deferred autoresearch ideas

- RSpec/HookArgument: implement full RuboCop parity autocorrect for explicit styles (`EnforcedStyle: each|example`) by rewriting/adding scope args on hook calls; current baseline only autocorrects implicit-style removals.
- RSpec/Focus: add metadata autocorrect parity (`:focus` and `focus: true` argument removal with surrounding comma/space handling) while keeping unsupported `focus` call cases uncorrected.

