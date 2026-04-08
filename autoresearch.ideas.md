# Deferred autoresearch ideas

- Rails/WhereNot: evaluate conservative RuboCop-aligned autocorrect for deterministic `where.not` rewrites with strict shape guards to avoid query-semantics drift.
- Rails/WhereEquals: evaluate safe autocorrect opportunities for unambiguous `where(id: x)` / equality forms that RuboCop rewrites without changing SQL semantics.
