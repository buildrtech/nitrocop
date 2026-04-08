# Deferred autoresearch ideas

- Rails/WhereRange: investigate why in-flight autocorrect patches are not surviving attribution-clean runs despite passing targeted tests; only keep once count script reliably reflects `supports_autocorrect`.
- Rails/DeprecatedActiveModelErrorsMethods: implement RuboCop-aligned partial autocorrect for `<<`, `clear`, and `keys` paths (`.add`, `.delete`, `.attribute_names`) with conservative skip for `errors.details[:key] << ...`.
