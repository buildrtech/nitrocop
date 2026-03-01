create :user
^^^^^^ FactoryBot/ConsistentParenthesesStyle: Prefer method call with parentheses
build :user
^^^^^ FactoryBot/ConsistentParenthesesStyle: Prefer method call with parentheses
build_list :user, 10
^^^^^^^^^^ FactoryBot/ConsistentParenthesesStyle: Prefer method call with parentheses
create_list :user, 10
^^^^^^^^^^^ FactoryBot/ConsistentParenthesesStyle: Prefer method call with parentheses
build_stubbed :user
^^^^^^^^^^^^^ FactoryBot/ConsistentParenthesesStyle: Prefer method call with parentheses

[
  (build :discord_server_role_response, position: 1),
   ^^^^^ FactoryBot/ConsistentParenthesesStyle: Prefer method call with parentheses
  (build :discord_server_role_response, position: 2),
   ^^^^^ FactoryBot/ConsistentParenthesesStyle: Prefer method call with parentheses
]
