class InventoriesController < ApplicationController
  before_action :set_adventure

  def show
    @items = @adventure.items
  end

  def new
    @items = Item.all.order( :name )
    @selected_items_ids = @adventure.item_ids
  end

  def create
    @adventure.item_ids = params[ 'selected_items' ]
    redirect_to inventory_url( @adventure ), notice: 'Item was successfully created.'
  end

  def destroy
    @item = Item.find( params[ 'item_id' ] )
    @adventure.items.delete( @item )
    respond_to do |format|
      format.html { redirect_to inventory_url( @adventure ), notice: 'Item was successfully destroyed.' }
      format.json { head :no_content }
    end
  end

  private
  # Use callbacks to share common setup or constraints between actions.
  def set_adventure
    @adventure = Adventure.find(params[:id])
  end


end
