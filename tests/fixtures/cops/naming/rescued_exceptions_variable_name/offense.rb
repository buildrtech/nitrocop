begin
  foo
rescue => ex
          ^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `ex` for rescued exceptions.
  bar
end
begin
  foo
rescue StandardError => err
                        ^^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `err` for rescued exceptions.
  bar
end
begin
  foo
rescue => exception
          ^^^^^^^^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `exception` for rescued exceptions.
  bar
end
begin
  something
rescue => @exception
          ^^^^^^^^^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `@exception` for rescued exceptions.
end
begin
  something
rescue => @@captured_error
          ^^^^^^^^^^^^^^^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `@@captured_error` for rescued exceptions.
end
begin
  something
rescue => $error
          ^^^^^^ Naming/RescuedExceptionsVariableName: Use `e` instead of `$error` for rescued exceptions.
end
