has_many :items, class_name: Item
                 ^^^^^^^^^^^^^^^^ Rails/ReflectionClassName: Use a string value for `class_name`.
belongs_to :author, class_name: User
                    ^^^^^^^^^^^^^^^^ Rails/ReflectionClassName: Use a string value for `class_name`.
has_one :profile, class_name: UserProfile.name
                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ReflectionClassName: Use a string value for `class_name`.
has_many :accounts, class_name: Account.to_s
                    ^^^^^^^^^^^^^^^^^^^^^^^^ Rails/ReflectionClassName: Use a string value for `class_name`.
belongs_to :account, class_name: Foo::Bar
                     ^^^^^^^^^^^^^^^^^^^^ Rails/ReflectionClassName: Use a string value for `class_name`.

# Local variable assigned a constant
class_name = Account
has_many :accounts, class_name: class_name
                    ^^^^^^^^^^^^^^^^^^^^^^ Rails/ReflectionClassName: Use a string value for `class_name`.

# Scope parameter with constant class_name
belongs_to :account, -> { distinct }, class_name: Account
                                      ^^^^^^^^^^^^^^^^^^^ Rails/ReflectionClassName: Use a string value for `class_name`.
