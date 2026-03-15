"text".to_s
       ^^^^ Lint/RedundantTypeConversion: Redundant `to_s` detected.
:sym.to_sym
     ^^^^^^ Lint/RedundantTypeConversion: Redundant `to_sym` detected.
42.to_i
   ^^^^ Lint/RedundantTypeConversion: Redundant `to_i` detected.
data.to_json.to_s
             ^^^^ Lint/RedundantTypeConversion: Redundant `to_s` detected.
foo.to_json(arg).to_s
                 ^^^^ Lint/RedundantTypeConversion: Redundant `to_s` detected.
