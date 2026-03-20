# Examples inside explicit begin..end blocks don't count.
# RuboCop's examples? matcher checks (begin ...) for implicit block bodies
# but not (kwbegin ...) for explicit begin..end blocks.
context 'with begin block' do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyExampleGroup: Empty example group detected.
  begin
    FileUtils.ln_s "source.txt", "link.txt"
    it "should handle symlinks" do
      expect(true).to be(true)
    end
  end
end
