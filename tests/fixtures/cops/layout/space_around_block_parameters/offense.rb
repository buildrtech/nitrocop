items.each { | x| puts x }
              ^ Layout/SpaceAroundBlockParameters: Space before first block parameter detected.

items.each { |x | puts x }
               ^ Layout/SpaceAroundBlockParameters: Space after last block parameter detected.

items.each { | x | puts x }
              ^ Layout/SpaceAroundBlockParameters: Space before first block parameter detected.
                ^ Layout/SpaceAroundBlockParameters: Space after last block parameter detected.

items.each { |x|puts x }
               ^ Layout/SpaceAroundBlockParameters: Space after closing `|` missing.

handler = proc {|s|cmd.call s}
                  ^ Layout/SpaceAroundBlockParameters: Space after closing `|` missing.

result = ->( x, y) { puts x }
            ^ Layout/SpaceAroundBlockParameters: Space before first block parameter detected.

result = ->(x, y  ) { puts x }
                ^^ Layout/SpaceAroundBlockParameters: Space after last block parameter detected.

result = ->(  a,  b, c) { puts a }
            ^^ Layout/SpaceAroundBlockParameters: Space before first block parameter detected.
                 ^ Layout/SpaceAroundBlockParameters: Extra space before block parameter detected.

items.each { |x,   y| puts x }
                 ^^ Layout/SpaceAroundBlockParameters: Extra space before block parameter detected.

items.each { |a, (x,  y), z| puts x }
                     ^ Layout/SpaceAroundBlockParameters: Extra space before block parameter detected.

items.each { |a,  (x,  y),  z| puts x }
                 ^ Layout/SpaceAroundBlockParameters: Extra space before block parameter detected.
                      ^ Layout/SpaceAroundBlockParameters: Extra space before block parameter detected.
                           ^ Layout/SpaceAroundBlockParameters: Extra space before block parameter detected.
