Integer(arg) rescue nil
^^^^^^^^^^^^^^^^^^^^^^^ Lint/SuppressedExceptionInNumberConversion: Use `Integer(arg, exception: false)` instead.
Float(arg) rescue nil
^^^^^^^^^^^^^^^^^^^^^ Lint/SuppressedExceptionInNumberConversion: Use `Float(arg, exception: false)` instead.
Complex(arg) rescue nil
^^^^^^^^^^^^^^^^^^^^^^^ Lint/SuppressedExceptionInNumberConversion: Use `Complex(arg, exception: false)` instead.
Kernel.Integer(arg) rescue nil
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/SuppressedExceptionInNumberConversion: Use `Kernel.Integer(arg, exception: false)` instead.

begin
^^^^^^^^^^^ Lint/SuppressedExceptionInNumberConversion: Use `Integer(arg, exception: false)` instead.
  Integer(arg)
rescue
  nil
end

begin
^^^^^^^^^^^ Lint/SuppressedExceptionInNumberConversion: Use `Kernel.Integer(arg, exception: false)` instead.
  Kernel.Integer(arg)
rescue ArgumentError
  nil
end
