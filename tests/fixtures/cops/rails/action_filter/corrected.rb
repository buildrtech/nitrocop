before_action :authenticate
after_action :cleanup
skip_before_action :login
around_action :wrap_in_transaction
