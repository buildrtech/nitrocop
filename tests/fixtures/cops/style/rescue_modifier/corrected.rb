x = begin
      something
    rescue
      nil
    end

y = begin
      foo.bar
    rescue
      false
    end

z = begin
      JSON.parse(str)
    rescue
      {}
    end
