[1, 2].each {|x| puts x}
            ^^ Layout/SpaceInsideBlockBraces: Space between { and | missing.
                       ^ Layout/SpaceInsideBlockBraces: Space missing inside }.
[1, 2].map {|x| x * 2}
           ^^ Layout/SpaceInsideBlockBraces: Space between { and | missing.
                     ^ Layout/SpaceInsideBlockBraces: Space missing inside }.
foo.select {|x| x > 1}
           ^^ Layout/SpaceInsideBlockBraces: Space between { and | missing.
                     ^ Layout/SpaceInsideBlockBraces: Space missing inside }.
