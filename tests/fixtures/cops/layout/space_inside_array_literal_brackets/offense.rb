[ 1, 2, 3 ]
^ Layout/SpaceInsideArrayLiteralBrackets: Space inside array literal brackets detected.
          ^ Layout/SpaceInsideArrayLiteralBrackets: Space inside array literal brackets detected.
[ :a, :b ]
^ Layout/SpaceInsideArrayLiteralBrackets: Space inside array literal brackets detected.
         ^ Layout/SpaceInsideArrayLiteralBrackets: Space inside array literal brackets detected.
x = [ "foo" ]
    ^ Layout/SpaceInsideArrayLiteralBrackets: Space inside array literal brackets detected.
            ^ Layout/SpaceInsideArrayLiteralBrackets: Space inside array literal brackets detected.
# Multiline array: space after [ when elements on same line (no_space default)
[ Element::Form, Element::Link,
^ Layout/SpaceInsideArrayLiteralBrackets: Space inside array literal brackets detected.
  Element::Cookie ]
                  ^ Layout/SpaceInsideArrayLiteralBrackets: Space inside array literal brackets detected.
# Multiple spaces after opening bracket
[  1, 2, 3]
^ Layout/SpaceInsideArrayLiteralBrackets: Space inside array literal brackets detected.
# Multiple spaces before closing bracket
[1, 2, 3   ]
           ^ Layout/SpaceInsideArrayLiteralBrackets: Space inside array literal brackets detected.
# Empty brackets with multiple spaces (should be empty offense)
[     ]
^^^^^^^ Layout/SpaceInsideArrayLiteralBrackets: Space inside empty array literal brackets detected.
# Empty brackets with newline (multiline empty)
[
^ Layout/SpaceInsideArrayLiteralBrackets: Space inside empty array literal brackets detected.
]
