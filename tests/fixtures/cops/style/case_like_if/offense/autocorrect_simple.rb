if status == 'active'
  run_active
elsif 'pending' == status
  run_pending
elsif status == 'archived'
  run_archived
else
  run_default
end
