arr.compact.map { |x| x.to_s }
            ^^^ Performance/ChainArrayAllocation: Use unchained `compact` and `map!` (followed by `return array` if required) instead of chaining `compact...map`.
arr.sort.map(&:to_s)
         ^^^ Performance/ChainArrayAllocation: Use unchained `sort` and `map!` (followed by `return array` if required) instead of chaining `sort...map`.
arr.uniq.map { |x| x.name }
         ^^^ Performance/ChainArrayAllocation: Use unchained `uniq` and `map!` (followed by `return array` if required) instead of chaining `uniq...map`.
arr.flatten.compact
            ^^^^^^^ Performance/ChainArrayAllocation: Use unchained `flatten` and `compact!` (followed by `return array` if required) instead of chaining `flatten...compact`.
arr.map { |x| x.to_i }.sort
                       ^^^^ Performance/ChainArrayAllocation: Use unchained `map` and `sort!` (followed by `return array` if required) instead of chaining `map...sort`.
arr.select { |x| x.valid? }.uniq
                            ^^^^ Performance/ChainArrayAllocation: Use unchained `select` and `uniq!` (followed by `return array` if required) instead of chaining `select...uniq`.
arr.reject(&:nil?).compact
                   ^^^^^^^ Performance/ChainArrayAllocation: Use unchained `reject` and `compact!` (followed by `return array` if required) instead of chaining `reject...compact`.
[1, 2, 3, 4].first(10).uniq
                       ^^^^ Performance/ChainArrayAllocation: Use unchained `first` and `uniq!` (followed by `return array` if required) instead of chaining `first...uniq`.
model.select { |item| item.foo }.select { |item| item.bar }
                                 ^^^^^^ Performance/ChainArrayAllocation: Use unchained `select` and `select!` (followed by `return array` if required) instead of chaining `select...select`.
# reverse returns new array, then map creates another
items.reverse.map { |x| x.to_s }
              ^^^ Performance/ChainArrayAllocation: Use unchained `reverse` and `map!` (followed by `return array` if required) instead of chaining `reverse...map`.
items.sort.reverse.map { |w| [w, format(w)] }
           ^^^^^^^ Performance/ChainArrayAllocation: Use unchained `sort` and `reverse!` (followed by `return array` if required) instead of chaining `sort...reverse`.
                   ^^^ Performance/ChainArrayAllocation: Use unchained `reverse` and `map!` (followed by `return array` if required) instead of chaining `reverse...map`.
# operator methods: + returns new array, uniq creates another
items.+(other).uniq
               ^^^^ Performance/ChainArrayAllocation: Use unchained `+` and `uniq!` (followed by `return array` if required) instead of chaining `+...uniq`.
items.-(excluded).compact
                  ^^^^^^^ Performance/ChainArrayAllocation: Use unchained `-` and `compact!` (followed by `return array` if required) instead of chaining `-...compact`.
