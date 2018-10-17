class ApplicationController < ActionController::Base
  protect_from_forgery with: :exception

  def set_adventure
    @user ||= current_user
    @adventure = @user&.current
  end

end
