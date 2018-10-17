class GameLogsController < ApplicationController

  before_action :set_adventure

  def show
    @log = @adventure.game_logs.includes( :page ).order( 'id DESC' ).paginate(:page => params[:page])
  end

  private

end
