Post.where(user_id: User.select(:id))

Comment.where(post_id: Post.select(:id))

Order.where(customer_id: Customer.select(:id))

Post.where(user_id: User.active.select(:id))

Post.where.not(user_id: User.active.select(:id))

Post.where.not(user_id: User.active.select(:id))

Post.rewhere('user_id IN (?)', User.active.select(:id))

Post.rewhere('user_id IN (?)', User.active.select(:id))
