items.each do |item| item / 5 end
           ^^ Standard/BlockSingleLineBraces: Prefer `{...}` over `do...end` for single-line blocks.

items.map do |x| x.to_s end
          ^^ Standard/BlockSingleLineBraces: Prefer `{...}` over `do...end` for single-line blocks.

[1, 2, 3].select do |n| n.odd? end
                 ^^ Standard/BlockSingleLineBraces: Prefer `{...}` over `do...end` for single-line blocks.

foo bar do |x| x end
        ^^ Standard/BlockSingleLineBraces: Prefer `{...}` over `do...end` for single-line blocks.
