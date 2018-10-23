class ItemsController < ApplicationController
  before_action :set_adventure, :set_items

  def show
  end

  # PATCH/PUT /items/1
  # PATCH/PUT /items/1.json
  def update
    respond_to do |format|
      p params
      @adventure.items = params[:inventory].to_hash
      if @adventure.save
        format.html { redirect_to items_path, notice: 'Liste mise à jour.' }
      else
        format.html { render items_path }
      end
    end
  end

  private
    # Use callbacks to share common setup or constraints between actions.

    def set_items
      @items = @adventure.items
      @items ||= {}
    end
end
