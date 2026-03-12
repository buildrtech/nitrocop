class UsersController < ApplicationController
  def show
    @user = User.find(params[:id])
    head :unprocessable_entity
  end

  def create
    nickname = params[:nickname]
    return if nickname&.blank?

    # StrongParametersExpect
    user_params = params.require(:user).permit(:name, :email)
    @user = User.new(user_params)

    if @user.save
      # RedirectBackOrTo
      redirect_back(fallback_location: root_path)
    else
      head :payload_too_large
    end
  end

  def update
    # ContentTag
    tag(:br)
    tag(:hr)
  end
end
