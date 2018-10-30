class BelongingsController < ApplicationController

  before_action :set_adventure

  def show
  end

  def add_gold
  end

  def add_gold_update
    amount = params[:gold_amount].to_i
    @adventure.increment( :gold, amount )

    if @adventure.save
      GameLog.gold_modification( @adventure, amount, :up )
      redirect_to play_adventures_path, notice: I18n.t( 'belongings.up.gold', count: amount )
    else
      render :add_gold
    end
  end

  def loose_gold
  end

  def loose_gold_update
    amount = params[:gold_amount].to_i
    @adventure.decrement( :gold, amount )

    if @adventure.save
      GameLog.gold_modification( @adventure, amount, :down )
      redirect_to play_adventures_path, notice: I18n.t( 'belongings.down.gold', count: amount )
    else
      render :loose_gold
    end
  end

end
