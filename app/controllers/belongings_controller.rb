class BelongingsController < ApplicationController

  before_action :set_adventure

  def show
  end

  def create
    if add_belonging
      redirect_to belongings_path
    else
      render :show
    end
  end

  def destroy
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

  private

  def belonging_params
    params.require(:belonging).permit(:name)
  end

  def add_belonging
    @belonging = Belonging.new( belonging_params )
    @belonging.adventure = @adventure

    @belongings_history = BelongingsHistory.new( belonging_params )
    @belongings_history.book = @adventure.book
    @belongings_history.user = @user

    @belonging.save && @belongings_history.save
  end

end
