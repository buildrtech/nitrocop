Rails.root.join("config", "database.yml").read
File.read("config/database.yml")
File.read(some_path)
Pathname.new("config").exist?
File.exist?("config/database.yml")
File.read(File.join(file_fixture_path, 'data.csv'))
File.read(File.join(some_dir, 'file.txt'))
YAML.safe_load(File.open(Rails.root.join("locale/en.yml")))
IO.copy_stream(File.open(Rails.root.join("public", "image.png")), string_io)
result = File.open(Rails.root.join('fixtures', 'data.html')).read
organization.profile_image = File.open(Rails.root.join("app/assets/images/2.png"))
record.avatar = File.open(Rails.root.join("uploads", "photo.png"))
item.attachment = IO.open(Rails.root.join("files", "report.pdf"))
