def self.expired_at
  1.week.since
end
def self.deadline
  3.months.ago
end
def self.started
  Time.now.yesterday
end
def self.future
  Date.today.tomorrow
end
def self.last_week
  7.days.before
end
