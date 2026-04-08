# Deferred autoresearch ideas

- Rails/WhereNot: evaluate conservative RuboCop-aligned autocorrect for deterministic `where.not` rewrites with strict shape guards to avoid query-semantics drift.
- Rails/EagerEvaluationLogMessage: wrap interpolated `Rails.logger.debug` arguments in lazy blocks with RuboCop-aligned selector-to-call-end replacement.
