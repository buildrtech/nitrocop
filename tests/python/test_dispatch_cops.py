#!/usr/bin/env python3
"""Tests for dispatch-cops.py helper functions."""
import importlib.util
from pathlib import Path


SCRIPT = Path(__file__).parents[2] / "scripts" / "dispatch-cops.py"
SPEC = importlib.util.spec_from_file_location("dispatch_cops", SCRIPT)
assert SPEC and SPEC.loader
gct = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gct)


def test_pascal_to_snake():
    assert gct.pascal_to_snake("NegatedWhile") == "negated_while"
    assert gct.pascal_to_snake("AmbiguousRange") == "ambiguous_range"
    assert gct.pascal_to_snake("HashLikeCase") == "hash_like_case"
    assert gct.pascal_to_snake("I18nLocaleTexts") == "i18n_locale_texts"
    assert gct.pascal_to_snake("HTTPClient") == "http_client"


def test_parse_cop_name():
    dept, name, snake = gct.parse_cop_name("Style/NegatedWhile")
    assert dept == "Style"
    assert name == "NegatedWhile"
    assert snake == "negated_while"


def test_dept_dir_name():
    assert gct.dept_dir_name("Style") == "style"
    assert gct.dept_dir_name("RSpec") == "rspec"
    assert gct.dept_dir_name("RSpecRails") == "rspec_rails"
    assert gct.dept_dir_name("FactoryBot") == "factory_bot"
    assert gct.dept_dir_name("Lint") == "lint"


def test_extract_source_lines():
    src = [
        "  6: def foo",
        ">>>  7: \tinclude Bar",
        "  8: end",
    ]
    lines, offense, idx = gct._extract_source_lines(src)
    assert len(lines) == 3
    assert "include Bar" in offense
    assert idx == 1


def test_extract_source_lines_no_offense():
    src = ["  1: x = 1", "  2: y = 2"]
    lines, offense, idx = gct._extract_source_lines(src)
    assert len(lines) == 2
    assert offense is None
    assert idx is None


def test_find_enclosing_structure_begin():
    lines = [
        "BEGIN {",
        "\tinclude Foo",
        "}",
    ]
    result = gct._find_enclosing_structure(lines, 1)
    assert result is not None
    assert "BEGIN" in result
    assert "PreExecutionNode" in result


def test_find_enclosing_structure_class():
    lines = [
        "class MyClass",
        "  def foo",
        "    bar",
        "  end",
        "end",
    ]
    result = gct._find_enclosing_structure(lines, 2)
    assert result is not None
    assert "method body" in result


def test_find_enclosing_structure_none():
    lines = ["x = 1"]
    result = gct._find_enclosing_structure(lines, 0)
    assert result is None


def test_find_enclosing_structure_top_level():
    lines = [
        "include Foo",
    ]
    result = gct._find_enclosing_structure(lines, 0)
    assert result is None


def test_extract_spec_excerpts():
    spec = '''
    it 'flags bad code' do
      expect_offense(<<~RUBY)
        x = 1
        ^^^^^ Lint/Foo: Bad.
      RUBY
    end

    it 'accepts good code' do
      expect_no_offenses(<<~RUBY)
        y = 2
      RUBY
    end
    '''
    result = gct.extract_spec_excerpts(spec)
    assert "expect_offense" in result
    assert "expect_no_offenses" in result


def test_extract_spec_excerpts_empty():
    result = gct.extract_spec_excerpts("# no specs here")
    assert result == "(no expect_offense blocks found)"


def test_detect_prism_pitfalls():
    source_with_hash = "if let Some(h) = node.as_hash_node() {"
    pitfalls = gct.detect_prism_pitfalls(source_with_hash)
    assert len(pitfalls) == 1
    assert "KeywordHashNode" in pitfalls[0]


def test_detect_prism_pitfalls_none():
    source = "fn check_node(&self) { }"
    pitfalls = gct.detect_prism_pitfalls(source)
    assert len(pitfalls) == 0


def test_format_with_diagnostics_omits_no_source_examples_when_diagnosed_exists():
    diagnostics = [
        {
            "kind": "fp",
            "loc": "repo: file.rb:1",
            "msg": "Bad spacing",
            "diagnosed": True,
            "detected": True,
            "offense_line": "%w[ a ]",
            "test_snippet": "%w[ a ]\n^ Layout/Foo: Bad spacing",
            "enclosing": None,
            "node_type": None,
            "source_context": "%w[ a ]",
        },
        {
            "kind": "fp",
            "loc": "repo: file.rb:2",
            "msg": "Bad spacing",
            "diagnosed": False,
            "reason": "no source context",
        },
    ]
    output = gct._format_with_diagnostics(
        "Layout/Foo",
        diagnostics,
        fp_examples=[
            {"loc": "repo: file.rb:1", "msg": "Bad spacing", "src": [">>> 1: %w[ a ]"]},
            {"loc": "repo: file.rb:2", "msg": "Bad spacing"},
        ],
        fn_examples=[],
    )
    assert "Omitted 1 pre-diagnostic FP example(s) with no source context" in output
    assert "(could not diagnose: no source context)" not in output
    assert "### Additional examples (not pre-diagnosed)" not in output


def test_format_with_diagnostics_keeps_no_source_examples_when_they_are_all_we_have():
    diagnostics = [
        {
            "kind": "fp",
            "loc": "repo: file.rb:2",
            "msg": "Bad spacing",
            "diagnosed": False,
            "reason": "no source context",
        },
    ]
    output = gct._format_with_diagnostics(
        "Layout/Foo",
        diagnostics,
        fp_examples=[{"loc": "repo: file.rb:2", "msg": "Bad spacing"}],
        fn_examples=[],
    )
    assert "(could not diagnose: no source context)" in output


if __name__ == "__main__":
    test_pascal_to_snake()
    test_parse_cop_name()
    test_dept_dir_name()
    test_extract_source_lines()
    test_extract_source_lines_no_offense()
    test_find_enclosing_structure_begin()
    test_find_enclosing_structure_class()
    test_find_enclosing_structure_none()
    test_find_enclosing_structure_top_level()
    test_extract_spec_excerpts()
    test_extract_spec_excerpts_empty()
    test_detect_prism_pitfalls()
    test_detect_prism_pitfalls_none()
    test_format_with_diagnostics_omits_no_source_examples_when_diagnosed_exists()
    test_format_with_diagnostics_keeps_no_source_examples_when_they_are_all_we_have()
    print("All tests passed.")
