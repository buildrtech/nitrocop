# belongs_to: default FK is {assoc_name}_id
belongs_to :user

belongs_to :author

belongs_to :category

# belongs_to with class_name: FK is still based on assoc name, not class_name
belongs_to :post, class_name: 'SpecialPost'

# has_many: default FK is {model_name}_id
class Book
  has_many :chapters
end

# has_one: default FK is {model_name}_id
class User
  has_one :profile
end

# has_and_belongs_to_many: default FK is {model_name}_id
class Book
  has_and_belongs_to_many :authors
end

# has_many with :as option: default FK is {as_value}_id
class Book
  has_many :chapters, as: :publishable
end

# has_one with :as option: default FK is {as_value}_id
class User
  has_one :avatar, as: :attachable
end

# has_many with string FK value
class Book
  has_many :chapters
end

# CamelCase class name
class UserProfile
  has_many :settings
end

# belongs_to with string association name
belongs_to "user"

# Multiline belongs_to — offense on the foreign_key: line
belongs_to :reviewer

# Multiline has_many — offense on the foreign_key: line
class Order
  has_many :items
end

# Multiline has_one with multiple options — offense on the foreign_key: line
class Account
  has_one :setting,
          class_name: 'AccountSetting'
end

# belongs_to with trailing block — RuboCop still flags these
belongs_to :layer_group, resource: GroupResource, writable: false do
  assign do |_groups, _layer_groups|
  end
end

# has_many inside ClassName.class_eval — RuboCop resolves the class from the receiver
Organization.class_eval do
  has_many :ai_models,
    class_name: "CustomAiModel",
    limited_by_pricing_plans: { limit_key: :ai_models }
end
