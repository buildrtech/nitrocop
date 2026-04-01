case status
when 'active'
  run_active
when 'pending'
  run_pending
when 'archived'
  run_archived
else
  run_default
end
