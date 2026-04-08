# Deferred autoresearch ideas

- Rails/RootJoinChain: stabilize and validate autocorrect for chained `Rails.root/public_path.join(...).join(...)` flattening to single multi-arg `join(...)`, with strict Rails-root anchoring and exact fixture newline parity.
- Rails/DotSeparatedKeys: finish robust autocorrect for `scope:` removal so singleton-hash calls do not leave trailing separators (`t('foo', )`), then re-validate against offense invariant.
