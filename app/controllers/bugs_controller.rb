class BugsController < ApplicationController

  before_action :set_adventure

  def new
    @page = @adventure.current_page
    @book = @page.book
    @bug = Bug.new( user: @user, page: @page )
  end

end
