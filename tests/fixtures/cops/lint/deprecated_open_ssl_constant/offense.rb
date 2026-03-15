OpenSSL::Cipher::AES.new(128, :GCM)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/DeprecatedOpenSSLConstant: Use `OpenSSL::Cipher` instead of `OpenSSL::Cipher::AES`.

OpenSSL::Cipher::Cipher.new('AES-256-ECB')
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/DeprecatedOpenSSLConstant: Use `OpenSSL::Cipher` instead of `OpenSSL::Cipher::Cipher`.

OpenSSL::Digest::SHA256.new
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/DeprecatedOpenSSLConstant: Use `OpenSSL::Digest` instead of `OpenSSL::Digest::SHA256`.

OpenSSL::Digest::SHA256.digest('foo')
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/DeprecatedOpenSSLConstant: Use `OpenSSL::Digest` instead of `OpenSSL::Digest::SHA256`.
