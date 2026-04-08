it { expect(false).not_to be_true }
it { expect(nil).not_to be_nil }
it { expect(0).not_to eq(1) }
expect {
  2 + 2
}.not_to raise_error
it { is_expected.not_to be_nil }
expect_it { not_to be_buffered }
expect_it { not_to be_streaming }
expect_it { not_to be_timed_out }
