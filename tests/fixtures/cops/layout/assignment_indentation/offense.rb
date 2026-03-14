x =
    1
    ^ Layout/AssignmentIndentation: Indent the first line of the right-hand-side of a multi-line assignment.
y =
      "hello"
      ^ Layout/AssignmentIndentation: Indent the first line of the right-hand-side of a multi-line assignment.
@var =
       :value
       ^ Layout/AssignmentIndentation: Indent the first line of the right-hand-side of a multi-line assignment.
x +=
       1
       ^ Layout/AssignmentIndentation: Indent the first line of the right-hand-side of a multi-line assignment.
y ||=
       "default"
       ^ Layout/AssignmentIndentation: Indent the first line of the right-hand-side of a multi-line assignment.
z &&=
       compute(value)
       ^ Layout/AssignmentIndentation: Indent the first line of the right-hand-side of a multi-line assignment.
@sanitized ||=
    vanity_converted(original)
    ^ Layout/AssignmentIndentation: Indent the first line of the right-hand-side of a multi-line assignment.
@@count +=
    1
    ^ Layout/AssignmentIndentation: Indent the first line of the right-hand-side of a multi-line assignment.
$global ||=
    compute_default
    ^ Layout/AssignmentIndentation: Indent the first line of the right-hand-side of a multi-line assignment.
CONST ||=
    "value"
    ^ Layout/AssignmentIndentation: Indent the first line of the right-hand-side of a multi-line assignment.
a, b =
if condition ; end
^^^^^^^^^^^^^^^^^^ Layout/AssignmentIndentation: Indent the first line of the right-hand-side of a multi-line assignment.
Module::CONST =
    "value"
    ^ Layout/AssignmentIndentation: Indent the first line of the right-hand-side of a multi-line assignment.
result[:key] =
    hash_from_xml(data)
    ^ Layout/AssignmentIndentation: Indent the first line of the right-hand-side of a multi-line assignment.
self.name =
    compute_name(input)
    ^ Layout/AssignmentIndentation: Indent the first line of the right-hand-side of a multi-line assignment.
obj.attr ||=
    default_value
    ^ Layout/AssignmentIndentation: Indent the first line of the right-hand-side of a multi-line assignment.
items[index] +=
    extra_count
    ^ Layout/AssignmentIndentation: Indent the first line of the right-hand-side of a multi-line assignment.
