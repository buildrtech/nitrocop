x = <<~EOC
  content
EOC
^^^ Naming/HeredocDelimiterNaming: Use meaningful heredoc delimiters.
y = <<~END
  content
END
^^^ Naming/HeredocDelimiterNaming: Use meaningful heredoc delimiters.
z = <<~EOS
  content
EOS
^^^ Naming/HeredocDelimiterNaming: Use meaningful heredoc delimiters.
q = <<-'+'
  content
+
^ Naming/HeredocDelimiterNaming: Use meaningful heredoc delimiters.
r = <<~`END`
  echo hello
END
^^^ Naming/HeredocDelimiterNaming: Use meaningful heredoc delimiters.
s = <<~END
END
# nitrocop-expect: 16:4 Naming/HeredocDelimiterNaming: Use meaningful heredoc delimiters.
t = <<~EOS
EOS
# nitrocop-expect: 18:4 Naming/HeredocDelimiterNaming: Use meaningful heredoc delimiters.
