User.find_by(name: "foo")

Post.find_by(slug: "hello-world")

Order.find_by(status: "pending")

# Multiline where.take — offense reported at where line, not take line
records.find_by(
  status: "active"
)
