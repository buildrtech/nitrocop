before do
  allow(foo).to receive(:one) { :two }
end

before do
  allow(foo).to receive("one") { :two }
end

before do
  foo.stub(:one) { :two }
end
