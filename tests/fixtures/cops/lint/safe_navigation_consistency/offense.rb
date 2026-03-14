foo.bar && foo&.baz
              ^^ Lint/SafeNavigationConsistency: Use `.` instead of unnecessary `&.`.
foo&.bar && foo&.baz
               ^^ Lint/SafeNavigationConsistency: Use `.` instead of unnecessary `&.`.
foo.bar || foo&.baz
              ^^ Lint/SafeNavigationConsistency: Use `.` instead of unnecessary `&.`.
foo&.bar || foo.baz
               ^ Lint/SafeNavigationConsistency: Use `&.` for consistency with safe navigation.
foo.bar && foobar.baz && foo&.qux
                            ^^ Lint/SafeNavigationConsistency: Use `.` instead of unnecessary `&.`.
foo.bar || foobar.baz || foo&.qux
                            ^^ Lint/SafeNavigationConsistency: Use `.` instead of unnecessary `&.`.
foo&.bar && foo&.baz || foo&.qux
               ^^ Lint/SafeNavigationConsistency: Use `.` instead of unnecessary `&.`.
foo.bar && foo.baz || foo&.qux
                         ^^ Lint/SafeNavigationConsistency: Use `.` instead of unnecessary `&.`.
foo&.bar && foo&.baz || foo.qux
               ^^ Lint/SafeNavigationConsistency: Use `.` instead of unnecessary `&.`.
foo > 5 && foo&.zero?
              ^^ Lint/SafeNavigationConsistency: Use `.` instead of unnecessary `&.`.
foo.bar && foo&.baz = 1
              ^^ Lint/SafeNavigationConsistency: Use `.` instead of unnecessary `&.`.
foo&.bar && foo&.baz = 1
               ^^ Lint/SafeNavigationConsistency: Use `.` instead of unnecessary `&.`.
