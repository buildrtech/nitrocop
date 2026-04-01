x = [1,
     2,
     3]
y = [:a,
     :b,
     :c]
z = ["x",
     "y"]

# Trailing comma creates implicit array — misaligned elements
config[:expiration] = valid_date,
                      config[:key_name] = key_name

t[:push] = "Commit changes",
           t[:pull] = "Update working copy",
           t[:switch] = "Open branch"

MAX_LENGTH = "x-max-length",
             QUEUE_TYPE = "x-queue-type"

# Array inside if/else within multi-assignment — arrays nested in
# control flow are NOT direct children of masgn, so should be checked
name, size = if condition
  ["first",
   target_size,
   target_storage]
else
  ["second",
   other_size,
   other_storage]
end

# Bracketed array inside multi-assignment with multiple RHS values —
# the array's parent is the implicit RHS array, not the masgn itself
res, ignored = [items.select { |f| !File.directory?(f) },
                items.select { |f| File.directory?(f) }], Dir.glob(".*")

# Rescue exception list misaligned
begin
  foo
rescue ArgumentError,
       RuntimeError,
       TypeError => e
  bar
end
