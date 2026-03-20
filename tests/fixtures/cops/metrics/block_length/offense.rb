items.each do |item|
^ Metrics/BlockLength: Block has too many lines. [26/25]
  x = 1
  x = 2
  x = 3
  x = 4
  x = 5
  x = 6
  x = 7
  x = 8
  x = 9
  x = 10
  x = 11
  x = 12
  x = 13
  x = 14
  x = 15
  x = 16
  x = 17
  x = 18
  x = 19
  x = 20
  x = 21
  x = 22
  x = 23
  x = 24
  x = 25
  x = 26
end

things.map do |t|
^ Metrics/BlockLength: Block has too many lines. [27/25]
  a = 1
  a = 2
  a = 3
  a = 4
  a = 5
  a = 6
  a = 7
  a = 8
  a = 9
  a = 10
  a = 11
  a = 12
  a = 13
  a = 14
  a = 15
  a = 16
  a = 17
  a = 18
  a = 19
  a = 20
  a = 21
  a = 22
  a = 23
  a = 24
  a = 25
  a = 26
  a = 27
end

[1, 2, 3].select do |n|
^ Metrics/BlockLength: Block has too many lines. [28/25]
  b = 1
  b = 2
  b = 3
  b = 4
  b = 5
  b = 6
  b = 7
  b = 8
  b = 9
  b = 10
  b = 11
  b = 12
  b = 13
  b = 14
  b = 15
  b = 16
  b = 17
  b = 18
  b = 19
  b = 20
  b = 21
  b = 22
  b = 23
  b = 24
  b = 25
  b = 26
  b = 27
  b = 28
end

fn = ->(x) do
     ^ Metrics/BlockLength: Block has too many lines. [26/25]
  a1 = 1
  a2 = 2
  a3 = 3
  a4 = 4
  a5 = 5
  a6 = 6
  a7 = 7
  a8 = 8
  a9 = 9
  a10 = 10
  a11 = 11
  a12 = 12
  a13 = 13
  a14 = 14
  a15 = 15
  a16 = 16
  a17 = 17
  a18 = 18
  a19 = 19
  a20 = 20
  a21 = 21
  a22 = 22
  a23 = 23
  a24 = 24
  a25 = 25
  a26 = 26
end

def with_super_block
  super do |value|
  ^^^^^^^^^^^^^^^^ Metrics/BlockLength: Block has too many lines. [26/25]
    v1 = 1
    v2 = 2
    v3 = 3
    v4 = 4
    v5 = 5
    v6 = 6
    v7 = 7
    v8 = 8
    v9 = 9
    v10 = 10
    v11 = 11
    v12 = 12
    v13 = 13
    v14 = 14
    v15 = 15
    v16 = 16
    v17 = 17
    v18 = 18
    v19 = 19
    v20 = 20
    v21 = 21
    v22 = 22
    v23 = 23
    v24 = 24
    v25 = 25
    v26 = 26
  end
end

payload = {
  check_records_exist: lambda {Hash.new(
                       ^^^^^^^^^^^^^^^^^^^^^ Metrics/BlockLength: Block has too many lines. [26/25]
    "code" => 200,
    "body" => {
      "records" => [
        {
          "actual_values" => [
            "v=spf1 a mx include:_spf.example.com ~all",
            "verification-token=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
          ],
          "address" => "example.com",
          "match" => false,
          "match_against" => "bogus record",
          "type" => "txt"
        },
        {
          "actual_values" => [
            "v=spf1 mx include:mail.example.com ~all"
          ],
          "address" => "app.example.com",
          "match" => false,
          "match_against" => "v=spf1 include:mail.example.com ~all",
          "type" => "txt"
        }
      ]
    }
  )}
}

# Block with heredoc in the middle and code after it.
# RuboCop includes heredoc content lines in the body count.
# Total body lines = 26 (heredoc content + surrounding code).
records.transaction do
^ Metrics/BlockLength: Block has too many lines. [26/25]
  sql = <<~SQL
    INSERT INTO items (name, value)
    SELECT :name, :value
    WHERE NOT EXISTS (
      SELECT 1 FROM items
      WHERE name = :name
    )
  SQL
  builder = build_query(sql)
  builder.where("name = :name")
  result = builder.exec(name: name, value: value)
  if result > 0
    update_counter
  end
  process_record(result)
  validate_output(result)
  log_activity(:insert, name)
  notify_watchers(name)
  cache_invalidate(name)
  audit_trail(:insert, name)
  refresh_index
  a1 = 1
  a2 = 2
  a3 = 3
  a4 = 4
  a5 = 5
end

# Qualified Struct.new: BenchmarkDriver::Struct.new is NOT a class constructor.
# RuboCop's class_constructor? uses (const {nil? cbase} :Struct), which only
# matches bare Struct or ::Struct, not qualified paths like Foo::Struct.
WarmupScript = BenchmarkDriver::Struct.new(:preludes, :script, :teardown) do
               ^ Metrics/BlockLength: Block has too many lines. [26/25]
  def render(result:)
    x1 = 1
    x2 = 2
    x3 = 3
    x4 = 4
    x5 = 5
    x6 = 6
    x7 = 7
    x8 = 8
    x9 = 9
    x10 = 10
    x11 = 11
    x12 = 12
    x13 = 13
    x14 = 14
    x15 = 15
    x16 = 16
    x17 = 17
    x18 = 18
    x19 = 19
    x20 = 20
    x21 = 21
    x22 = 22
    x23 = 23
    x24 = 24
  end
end

# proc do...end blocks are regular blocks, NOT exempt.
display = proc do |code|
          ^^^^ Metrics/BlockLength: Block has too many lines. [26/25]
  tree = nil
  p1 = 1
  p2 = 2
  p3 = 3
  p4 = 4
  p5 = 5
  p6 = 6
  p7 = 7
  p8 = 8
  p9 = 9
  p10 = 10
  p11 = 11
  p12 = 12
  p13 = 13
  p14 = 14
  p15 = 15
  p16 = 16
  p17 = 17
  p18 = 18
  p19 = 19
  p20 = 20
  p21 = 21
  p22 = 22
  p23 = 23
  p24 = 24
  p25 = 25
end
