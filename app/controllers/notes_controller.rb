class NotesController < ApplicationController

  before_action :set_adventure

  def edit
  end

  def update
    notes = params[:notes]

    respond_to do |format|
      if @adventure.update( notes: notes )
        format.html { redirect_to adventure_play_url( @adventure ), notice: I18n.t( 'notes.update.success' ) }
      else
        format.html { render :edit }
      end
    end
  end

  private
  # Use callbacks to share common setup or constraints between actions.
  def set_adventure
    @adventure = Adventure.find(params[:id])
  end

end