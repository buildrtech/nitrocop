# Deferred autoresearch ideas

- Rails/RootJoinChain: stabilize and validate autocorrect for chained `Rails.root/public_path.join(...).join(...)` flattening to single multi-arg `join(...)`, with ownership-safe traversal and fixture newline parity.
- Rails/I18nLazyLookup: verify and harden autocorrect parity in both `lazy` and `explicit` styles (quoted string/symbol edge cases, controller path scoping).
