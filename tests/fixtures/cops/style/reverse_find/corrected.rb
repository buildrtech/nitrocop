array.rfind { |x| x > 0 }

[1, 2, 3].rfind(&:odd?)

items.rfind { |i| i.valid? }

dependabot_versions
  .sort
  .rfind { |version| version > 1 }
