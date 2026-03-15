User.find_by(name: "foo")
User.find_by(email: "test@test.com")
find_by_name("foo")
User.where(name: "foo")
User.find(1)
# Argument count mismatch: 2 columns but 3 args
User.find_by_name_and_email(name, email, token)
# Argument count mismatch: 2 columns but 1 arg
User.find_by_name_and_email(name)
# Hash argument means it's not a dynamic finder
User.find_by_id(limit: 1)
# Splat argument
User.find_by_scan(*args)
# Mixed args with splat
User.find_by_foo_and_bar(arg, *args)
# Custom method with keyword hash args (not a dynamic finder)
GithubPullRequest.find_by_github_identifiers(id: 123, url: "http://example.com")
# Multiple keyword arguments with hash
Post.find_by_title_and_id("foo", limit: 1)
# Explicit hash literal argument (not a dynamic finder column lookup)
service.find_by_ids({ "category" => [1, 2] })
# Receiverless find_by_* outside of ActiveRecord class (no offense)
class Service
  def lookup
    find_by_name("foo")
  end
end
# Receiverless find_by_* in class not inheriting AR
class Utility < BaseService
  def lookup
    find_by_name("foo")
  end
end
