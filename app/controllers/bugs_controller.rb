class BugsController < ApplicationController

  before_action :set_adventure

  def new
    @page = @adventure.current_page
    @book = @page.book
    @bug = Bug.new( user: @user, page: @page )
  end

  def create
    @bug = Bug.new( bug_params )
    @bug.user_id = @user.id

    if @bug.save
      redirect_to new_bug_path, notice: 'Merci pour votre contribution'
    else
      render :new
    end
  end

  private

  def bug_params
    params.require(:bug).permit(:info, :page_id)
  end

end
