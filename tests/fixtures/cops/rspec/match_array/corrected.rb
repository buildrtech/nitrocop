it { is_expected.to contain_exactly(content1, content2) }
it { is_expected.to contain_exactly(*content1, content2) }
it { is_expected.to contain_exactly(1, 2, 3) }
