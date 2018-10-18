class NotesController < ApplicationController

  before_action :set_adventure

  def edit
  end

  def update
    notes = params[:notes]

    respond_to do |format|
      if @adventure.update( notes: notes )
        format.html { redirect_to play_adventures_path, notice: I18n.t( 'notes.update.success' ) }
      else
        format.html { render :edit }
      end
    end
  end

end
