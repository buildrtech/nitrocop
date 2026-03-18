items.each do |x|
  puts x
  end
  ^^^ Layout/BlockAlignment: Align `end` with the start of the line where the block is defined.
items.map do |x|
  x * 2
    end
    ^^^ Layout/BlockAlignment: Align `end` with the start of the line where the block is defined.
[1, 2].select do |x|
  x > 1
      end
      ^^^ Layout/BlockAlignment: Align `end` with the start of the line where the block is defined.
# FN: end aligns with RHS of assignment (call expression) instead of LHS
answer = prompt.select("Pick one") do |menu|
           menu.choice "A"
         end
         ^^^ Layout/BlockAlignment: Align `end` with the start of the line where the block is defined.
# FN: brace block } misaligned — } at col 4, lambda{ at col 8
req = Rack::MockRequest.new(
  show_status(
        lambda{|env|
          env["rack.showstatus.detail"] = "gone too meta."
          [404, { "content-type" => "text/plain", "content-length" => "0" }, []]
    }))
    ^ Layout/BlockAlignment: Align `}` with the start of the line where the block is defined.
# FN: do..end block misaligned in multi-arg call
assert_called_with(
  ActiveRecord::Tasks::DatabaseTasks, :structure_dump,
  ["task_dump",
   "--result-file",
   filename]
) do
        ActiveRecord::Tasks::DatabaseTasks.structure_dump(
          @configuration.merge("sslca" => "ca.crt"),
          filename)
        end
        ^^^ Layout/BlockAlignment: Align `end` with the start of the line where the block is defined.
