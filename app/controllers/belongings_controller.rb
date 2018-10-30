class BelongingsController < ApplicationController

  before_action :set_adventure

  def show
  end

  def add_gold
  end

  def add_gold_update
    @adventure.increment( :gold, params[:gold_amount].to_i )

    if @adventure.save
      redirect_to play_adventures_path, notice: 'Or ajouté.'
    else
      render :add_gold
    end
  end

  def loose_gold
  end

  def loose_gold_update
    @adventure.decrement( :gold, params[:gold_amount].to_i )

    if @adventure.save
      redirect_to play_adventures_path, notice: 'Or supprimé.'
    else
      render :loose_gold
    end
  end

end
