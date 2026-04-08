# Deferred autoresearch ideas

- Rails/MigrationClassName: add conservative autocorrect only for underscore-style class names (`add_users` -> `AddUsers`) while leaving filename-mismatch and non-underscore variants diagnostic-only.
- Rails/I18nLazyLookup: verify and harden autocorrect parity in both `lazy` and `explicit` styles (quoted string/symbol edge cases, controller path scoping).
