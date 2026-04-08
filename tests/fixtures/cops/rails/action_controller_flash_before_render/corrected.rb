class PostsController < ApplicationController
  def update
    flash.now[:alert] = "Update failed"
    render :edit
  end
end

class UsersController < ApplicationController
  def create
    flash.now[:notice] = "Created"
    render :new
  end
end

class OrdersController < ApplicationController
  def show
    flash.now[:error] = "Not found"
    render :not_found
  end
end

class ItemsController < ApplicationController
  def create
    respond_to do |format|
      format.js do
        flash.now[:error] = "Something went wrong"
        render js: "window.location.href = '/'"
      end
    end
  end
end

class EventsController < ApplicationController
  def update
    respond_to do |format|
      format.html do
        flash.now[:notice] = "Updated"
        render :edit
      end
    end
  end
end

# Implicit render: flash in a def with no explicit render call
class HomeController < ApplicationController
  def create
    flash.now[:alert] = "msg"
  end
end

# flash before render with ::ApplicationController (top-level constant)
class PagesController < ::ApplicationController
  def index
    flash.now[:notice] = "Welcome"
    render :index
  end
end

# flash before render with ::ActionController::Base
class ApiController < ::ActionController::Base
  def show
    flash.now[:alert] = "Not found"
    render :show
  end
end

# flash in if-block with render at outer level
class RecordsController < ApplicationController
  def create
    if condition
      do_something
      flash.now[:alert] = "msg"
    end

    render :index
  end
end

# before_action block with flash and render
class SettingsController < ApplicationController
  before_action do
    flash.now[:alert] = "msg"
    render :index
  end
end

# FN fix: redirect_to inside respond_to format block is NOT a direct sibling redirect
class TasksController < ApplicationController
  def respond_to_not_found
    flash.now[:warning] = "Not available"
    respond_to do |format|
      format.html { redirect_to(root_path) }
      format.js   { render plain: 'window.location.reload();' }
    end
  end
end

# FN fix: modifier-unless flash before render at def level
class SessionsController < ApplicationController
  def failure
    flash.now[:error] = "Auth error" unless params[:message].nil?
    render action: :new
  end
end

# FN fix: flash inside unless block with render in method after unless
class TagsController < ApplicationController
  def create
    unless type_valid?
      flash.now[:error] = "Please provide a category."
      return
    end
    process_tag
    render action: "new"
  end
end

# FN fix: modifier-if flash inside else branch with render as sibling in same branch
class InvitationsController < ApplicationController
  def update
    if @invitation.save
      redirect_to @invitation
    else
      flash.now[:error] = "Invalid email" if @invitation.invitee_email.blank?
      render action: "show"
    end
  end
end

# FN fix: flash in elsif branch before render in same branch
class PreferencesController < ApplicationController
  def update
    if valid_params?
      if @user.update(params[:user])
        redirect_to config_path
      else
        flash.now[:error] = "Error updating preferences"
        render :edit
      end
    else
      announce_bad_data
      render :edit
    end
  end
end

# FN fix: flash in else branch before respond_to with render
class CommentsController < ApplicationController
  def create
    if @comment.save
      process_comment
    else
      flash.now[:error] = "Comment cannot be empty"
    end
    respond_to do |format|
      format.html { redirect_to listing_path }
      format.js { render layout: false }
    end
  end
end

# FN fix: flash in else branch with render as direct outer sibling
class AspectController < ApplicationController
  def update
    if @aspect.update(params)
      flash.now[:notice] = "Updated"
    else
      flash.now[:error] = "Failed to update"
    end
    render json: { id: @aspect.id }
  end
end

# FN fix: flash alone in each block — implicit render
class NotificationsController < ApplicationController
  def flash_messages
    get_messages.each do |message|
      flash.now[message[:type]] = { body: message[:body] }
    end
  end
end

# FN fix: flash in multi-statement block body — implicit render (outer redirect not visible)
class CallbacksController < ApplicationController
  def execute
    service.on_success do
      count = service.result
      flash.now[:notice] = "Processed items"
    end
    redirect_to callbacks_path
  end
end

# FN fix: flash in deeply nested single-child if — parent else has render
class StatusController < ApplicationController
  def check_status
    if primary_condition?
      if secondary_condition?
        if user_present?
          do_cleanup
          flash.now[:error] = "Status issue"
        end
      else
        render html: "Fallback content"
      end
    end
  end
end

# FN fix: flash inside unless body in def-with-rescue (Prism wraps body as BeginNode)
# The unless node's outer siblings include an if/else with render.
class UploadsController < ApplicationController
  def create
    unless valid_file?
      flash.now[:error] = "Invalid file"
      render :upload_form, status: :unprocessable_entity
      return
    end
    if save_result?
      redirect_to uploads_path
    else
      flash.now[:error] = "Save failed"
      render :upload_form, status: :unprocessable_entity
    end
  rescue UploadError => e
    flash.now[:error] = e.message
    render :upload_form
  end
end

# FN fix: flash in if body inside def-with-rescue, render in right siblings of if
class ProfileController < ApplicationController
  def update
    if invalid_input?
      flash.now[:error] = "Invalid input"
      return
    end
    if save_record?
      redirect_to profile_path
    else
      render :edit, status: :unprocessable_entity
    end
  rescue StandardError => e
    redirect_to profile_path
  end
end

# RuboCop's def_node_search :action_controller? matches ANY reference to
# ApplicationController/ActionController::Base in the class subtree, not just superclass
class Widget < ActiveRecord::Base
  VIEWS = ActionController::Base.view_paths

  def store_in_flash
    flash.now[:key] = "value"
  end
end

# FN fix: flash inside case/when branches before redirect at method level
class ArticlesController < ApplicationController
  def cancelvote
    @article.unvote_by current_user
    case @article.vote_registered?
    when true
      flash.now[:notice] = %(Could not cancel your vote for the article "#{@article.title}")
    when false
      flash.now[:notice] = %(Cancelled your vote for the article "#{@article.title}")
    when nil
      flash.now[:error] = 'Can not cancel when you have not voted for this article'
    end
    redirect_to article_path(@article)
  end
end

# FN fix: lambda hash values should be checked like regular block bodies
class AgentsController < ApplicationController
  def create
    handle_crud(
      on_invalid: lambda {
        ensure_auth_and_display
        return render_aspace_partial partial: "agents/new" if inline?
        return render action: :new
      },
      on_valid: lambda { |id|
        flash.now[:success] = t("agent._frontend.messages.created")

        if @agent["is_slug_auto"] == false &&
           @agent["slug"].nil? &&
           params["agent"] &&
           params["agent"]["is_slug_auto"] == "1"
          flash.now[:warning] = t("slug.autogen_disabled")
        end

        return render json: @agent.to_hash if inline?
        if params.key?(:plus_one)
          return redirect_to(controller: :agents, action: :new, agent_type: @agent_type)
        end

        redirect_to(controller: :agents, action: :edit, id: id, agent_type: @agent_type)
      }
    )
  end
end

# FN fix: stabby lambdas nested in keyword hashes should also be visited
class DigitalObjectsController < ApplicationController
  def create
    handle_crud(
      :on_invalid => ->() {
        return render_aspace_partial :partial => "new" if inline?
        render :action => "new"
      },
      :on_valid => ->(id) {
        flash.now[:success] = t("digital_object._frontend.messages.created", digital_object_title: clean_mixed_content(@digital_object.title))

        if @digital_object["is_slug_auto"] == false &&
           @digital_object["slug"] == nil &&
           params["digital_object"] &&
           params["digital_object"]["is_slug_auto"] == "1"
          flash.now[:warning] = t("slug.autogen_disabled")
        end

        return render :json => @digital_object.to_hash if inline?
        redirect_to(
          :controller => :digital_objects,
          :action => :edit,
          :id => id
        )
      }
    )
  end
end

# FN fix: on_invalid lambdas nested in call arguments should be checked
class UsersController < ApplicationController
  def update
    update_user(
      :on_invalid => ->() {
        flash.now[:error] = t("user._frontend.messages.error_update")
        render :action => "edit"
      },
      :on_valid => ->(id) {
        redirect_to :action => :index
      }
    )
  end
end

# FN fix: local lambda assignments should be visited, not just statement-level blocks
class SessionsController < ApplicationController
  def authenticate_sensitive
    on_success = lambda do
      session[:last_authenticated_at] = Time.now
    end
    on_failure = lambda do
      flash.now[:danger] = I18n.t("users.edit.sensitive.failure")
    end

    render :edit
  end
end

# FN fix: explicit begin/rescue bodies should see render in rescue clauses,
# but not render after the begin/end block
class AdviceController < ApplicationController
  def save_advice
    begin
      unless params[:advice].nil?
        params[:advice].keys.each do |advice_key|
          QuestionAdvice.update(advice_key, advice: params[:advice][advice_key.to_sym][:advice])
        end
        flash.now[:notice] = "The advice was successfully saved!"
      end
    rescue ActiveRecord::RecordNotFound
      render action: "edit_advice", id: params[:id]
    end
    redirect_to action: "edit_advice", id: params[:id]
  end
end
