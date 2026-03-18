x = '%<name>s is %<age>d'
y = '%s'
z = 'hello world'
a = '%%s'
b = '%<greeting>s %<target>s'
c = '%d'
d = '%c/%u |%b%i| %e'
e = "%b %d %l:%M%P"
g = '%s %s %d'
# Incomplete template token: %{ without closing }name
h = '%{'
i = ['%{', '}']
# Incomplete annotated token: %< without closing >
j = '%<'
# Interpolated string with %{ that doesn't form complete token
k = "%{#{keyword}}"
# Unannotated tokens in interpolated format strings are NOT flagged
# because str parts inside dstr don't have format context in RuboCop
l = format("#{prefix} %s %s", a, b)
m = sprintf("#{prefix} %d %d", a, b)
# Unannotated in heredoc used as format string
n = format(<<~FMT, a, b)
  %s
  %s
FMT
# Unannotated tokens in non-format-context string
o = "contains %s and %d tokens"
# Strings inside backtick (xstr) context are skipped
p = `curl -w '%{http_code}' http://example.com`
q = `echo %{name} %s`
