x = [1,
     2,
     3]
y = [:a,
     :b,
     :c]
z = [1, 2, 3]
[1,
 2]

# Multiple elements per line, first element aligned
owned_classes = [
  Status, StatusPin, MediaAttachment, Poll, Report, Tombstone, Favourite,
  Follow, FollowRequest, Block, Mute,
  AccountModerationNote, AccountPin, AccountStat, ListAccount,
  PollVote, Mention, AccountDeletionRequest, AccountNote,
  Appeal, TagFollow
]

# Element is not the first token on its line (}, { pattern)
actions = [
  {
    edit: { range: range, newText: text }
  }, {
    edit: { range: other_range, newText: other_text }
  }
]

# Multi-assignment continuation — not an array literal
count, registry = 0,
{id: {}, value: {}, position:{}, type: {}}

file, old, ret = File.new(log, 'w'),
$stdout.dup, nil

e1, e2, e3, e4 = create(:enterprise), create(:enterprise), create(:enterprise),
create(:enterprise)

vetho, vethi = [local_ip.network.to_s,
                local_ip.next_sib.network.to_s]

# Rescue exception list aligned with first exception
begin
  foo
rescue ArgumentError,
       RuntimeError,
       TypeError => e
  bar
end

# Single rescue exception (no alignment needed)
begin
  foo
rescue ArgumentError => e
  bar
end
