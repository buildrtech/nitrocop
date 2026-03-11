# Copyright 2025 Acme Inc.

class EndlessMethods
  def condition = true
  def fallback = false

  def ambiguous_if = true if condition
  def ambiguous_unless = true unless fallback
  def ambiguous_or = true or fallback
end
