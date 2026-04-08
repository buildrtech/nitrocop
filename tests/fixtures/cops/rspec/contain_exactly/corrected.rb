it { is_expected.to match_array(array1 + array2) }
it { is_expected.to match_array([1, 2, 3]) }
it { is_expected.to match_array(a.merge(b)) }
