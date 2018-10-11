class ItemsController < ApplicationController
  before_action :set_adventure, :set_items

  def show
  end

  # PATCH/PUT /items/1
  # PATCH/PUT /items/1.json
  def update
    respond_to do |format|
      @adventure.items = params[:inventory]
      if @adventure.save
        format.html { redirect_to adventure_items_path( @adventure ), notice: 'Liste mise à jour.' }
      else
        format.html { render adventure_items_path( @adventure ) }
      end
    end
  end

  private
    # Use callbacks to share common setup or constraints between actions.
    def set_adventure
      @adventure = Adventure.find(params[:adventure_id])
    end

    def set_items
      @items = @adventure.items
      @items ||= {}
    end
end
