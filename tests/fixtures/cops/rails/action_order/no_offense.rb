class UsersController < ApplicationController
  def index
  end

  def show
  end

  def new
  end

  def edit
  end

  def create
  end

  def update
  end

  def destroy
  end
end

# Actions after bare `protected` should not be checked
class ProtectedController < ApplicationController
  def show; end
  protected
  def index; end
end

# Actions after bare `private` should not be checked
class PrivateController < ApplicationController
  def show; end
  private
  def index; end
end

# Inline `protected def` should not be checked
class InlineProtectedController < ApplicationController
  def show; end
  protected def index; end
end

# Inline `private def` should not be checked
class InlinePrivateController < ApplicationController
  def show; end
  private def index; end
end

# Mixed: public actions in order, then private out of order
class MixedController < ApplicationController
  def index; end
  def show; end
  private
  def destroy; end
  def create; end
end

# Actions made private via `private :method_name` should not be checked
class SymbolPrivateController < ApplicationController
  def show; end
  def index; end
  private :index
end

# Actions made protected via `protected :method_name` should not be checked
class SymbolProtectedController < ApplicationController
  def show; end
  def index; end
  protected :index
end

# Multiple actions made private via `private :action1, :action2`
class MultiSymbolPrivateController < ApplicationController
  def show; end
  def index; end
  private :index, :show
end

# private :method after the def, mixed with public actions in order
class MixedSymbolPrivateController < ApplicationController
  def index; end
  def destroy; end
  def show; end
  private :destroy, :show
end

# Consecutive-pair comparison: actions in partial expected order (new, edit, create) — no offense
# because each action is in order relative to its immediate predecessor.
class PartialOrderController < ApplicationController
  def new
  end

  def edit
  end

  def create
  end
end

# Module concerns should not be checked for action order
module LabelConcern
  def create
    model.update_labels(params[:labels])
  end

  def index
    @labels = model.label_list
  end
end
