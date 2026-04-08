# Deferred autoresearch ideas

- Rails/FreezeTime: add RuboCop-aligned autocorrect for currently-detected `travel_to(Time.now/current/Time.zone.now)` forms, including `&block` pass-through (`freeze_time(&block)`), while preserving existing offense detection scope.
- Rails/WhereMissing: evaluate RuboCop-aligned autocorrect opportunities for straightforward `left_joins(...).where(...: nil)` forms to `where.missing(...)` with strict receiver/arg-shape guards.
- Rails/I18nLazyLookup (deferred): revisit only after higher-confidence selector/range rewrites are exhausted; prior attempts were confounded by fixture/config parity drift.
