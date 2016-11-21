class GameLogsController < ApplicationController

  before_action :set_adventure

  def show
    @log = @adventure.game_logs.includes( :page ).order( 'id DESC' )
  end

  private

  def set_adventure
    @adventure = Adventure.find(params[:id])
  end


end
