Model.pluck(:name)

User.pluck(:email)

Post.pluck(:title)

Model.pluck(:name)

User.pluck(:email)

Model.select(:name).where(active: true).map(&:name)

pluck(:name)
