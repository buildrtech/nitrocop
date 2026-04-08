# Deferred autoresearch ideas

- Rails/ReflectionClassName: add safe autocorrect for `class_name:` values that are constants or `Constant.name`/`Constant.to_s` (rewrite to string literal), while leaving dynamic expressions uncorrected.
- Rails/I18nLazyLookup: add conservative autocorrect for explicit `I18n.t("scope.key")`/`t("scope.key")` forms where deterministic lazy lookup shortening is RuboCop-aligned; skip dynamic/interpolated keys.
- Rails/WhereMissing: evaluate RuboCop-aligned autocorrect opportunities for straightforward `left_joins(...).where(...: nil)` forms to `where.missing(...)` with strict receiver/arg-shape guards.
