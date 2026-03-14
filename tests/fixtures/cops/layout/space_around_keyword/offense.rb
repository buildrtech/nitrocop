if(x)
^^ Layout/SpaceAroundKeyword: Space after keyword `if` is missing.
  y
end
unless(x)
^^^^^^ Layout/SpaceAroundKeyword: Space after keyword `unless` is missing.
  y
end
while(x)
^^^^^ Layout/SpaceAroundKeyword: Space after keyword `while` is missing.
  y
end
x = IO.read(__FILE__)rescue nil
                     ^^^^^^ Layout/SpaceAroundKeyword: Space before keyword `rescue` is missing.
x = 1and 2
     ^^^ Layout/SpaceAroundKeyword: Space before keyword `and` is missing.
x = 1or 2
     ^^ Layout/SpaceAroundKeyword: Space before keyword `or` is missing.
x = a.to_s; y = b.to_s; z = c if(true)
                              ^^ Layout/SpaceAroundKeyword: Space after keyword `if` is missing.
