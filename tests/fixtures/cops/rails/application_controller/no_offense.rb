class UsersController < ApplicationController
end

class ApplicationController < ActionController::Base
end

class PostsController < ApplicationController
end

ApplicationController = Class.new(ActionController::Base)

class SomeModel < ActiveRecord::Base
end
