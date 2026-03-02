@current_user ||= User.find_by!(id: session[:user_id])
current_user ||= User.find_by(id: session[:user_id])
@current_user ||= User.find_by(id: session[:user_id]) || User.anonymous
@current_user ||= session[:user_id] ? User.find_by(id: session[:user_id]) : nil
@current_user = User.find_by(id: session[:user_id])
@current_user &&= User.find_by(id: session[:user_id])
@follow_request ||= FollowRequest.find_by(target_account: @account, uri: uri) unless uri.nil?
@follow ||= Follow.find_by(target_account: @account, uri: uri) if active?

# Safe navigation — &.find_by is excluded by RuboCop
@project ||= budget&.projects&.find_by(id: params[:project_id])

# Inside if/else block — RuboCop skips if any if ancestor
if params[:type] == "Post"
  @record ||= Post.find_by(id: params[:id])
else
  @record ||= Comment.find_by(id: params[:id])
end

# Inside if block
if @product
  @property ||= @product.properties.find_by(id: params[:id])
end

# Multiline modifier if
@collection ||= Collection.find_by(name: params[:collection_id]) if
    params[:collection_id]
