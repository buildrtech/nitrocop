class UsersController < ApplicationController
end

class PostsController < ApplicationController
end

class AdminController < ApplicationController
end

class MyController < ApplicationController; end

module Nested
  class MyController < ApplicationController; end
end

class Nested::MyController < ApplicationController; end

MyController = Class.new(ApplicationController)

Class.new(ApplicationController) {}

# stub_const with ApplicationController in string argument — should fire because
# `ApplicationController` is only inside a string, not a constant assignment LHS.
stub_const("Trestle::ApplicationController", Class.new(ApplicationController))
