File.read(Rails.root.join("config", "database.yml"))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: `Rails.root` is a `Pathname`, so you can use `Rails.root.join(...).read` instead.

File.exist?(Rails.root.join("tmp", "restart.txt"))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: `Rails.root` is a `Pathname`, so you can use `Rails.root.join(...).exist?` instead.

File.delete(Rails.root.join("tmp", "pids", "server.pid"))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: `Rails.root` is a `Pathname`, so you can use `Rails.root.join(...).delete` instead.

File.join(Rails.root, "config", "initializers", "action_mailer.rb")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: `Rails.root` is a `Pathname`, so you can use `Rails.root.join` instead of `File.join(Rails.root, ...)`.

Dir.glob(Rails.root.join("db", "**", "*.rb"))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: `Rails.root` is a `Pathname`, so you can use `Rails.root.join(...).glob` instead.

File.join(Rails.public_path, "uploads", "photo.jpg")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: `Rails.public_path` is a `Pathname`, so you can use `Rails.public_path.join` instead of `File.join(Rails.public_path, ...)`.

File.exist?(Rails.public_path.join("assets", "app.css"))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: `Rails.public_path` is a `Pathname`, so you can use `Rails.public_path.join(...).exist?` instead.

File.read(Rails.public_path.join("robots.txt"))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: `Rails.public_path` is a `Pathname`, so you can use `Rails.public_path.join(...).read` instead.

File.open(Rails.root.join("public", filename))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: `Rails.root` is a `Pathname`, so you can use `Rails.root.join(...).open` instead.

File.open(Rails.root.join("db", "schema.rb"))
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: `Rails.root` is a `Pathname`, so you can use `Rails.root.join(...).open` instead.

File.open(Rails.root.join("config", "secrets.yml"), "r")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: `Rails.root` is a `Pathname`, so you can use `Rails.root.join(...).open` instead.

blob = {io: File.open(Rails.root.join("public", "photo.png"))}
            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RootPathnameMethods: `Rails.root` is a `Pathname`, so you can use `Rails.root.join(...).open` instead.
