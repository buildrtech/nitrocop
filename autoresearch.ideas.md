# Deferred autoresearch ideas

- Performance/Caller: add RuboCop-style autocorrect (`caller.first` / `caller[n]` / `caller_locations...`) to `caller(m..m).first` with integer-index arithmetic parity.
- Performance/Count: port RuboCop autocorrect for `select/reject/filter/find_all ... count/size/length` chains, including reject-negation handling and block-pass parity.
- Rails/WhereRange: revisit conservative autocorrect (`where("x >= ?", a)` -> `where(x: a..)` and related shapes) only after attribution-clean run succeeds end-to-end with stable metrics capture.
