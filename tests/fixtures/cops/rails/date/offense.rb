Date.today
     ^^^^^ Rails/Date: Use `Date.current` instead of `Date.today`.

x = Date.today
         ^^^^^ Rails/Date: Use `Date.current` instead of `Date.today`.

if Date.today > deadline
        ^^^^^ Rails/Date: Use `Date.current` instead of `Date.today`.
end

::Date.today
       ^^^^^ Rails/Date: Use `Date.current` instead of `Date.today`.

value.to_time_in_current_zone
      ^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Date: `to_time_in_current_zone` is deprecated. Use `in_time_zone` instead.
