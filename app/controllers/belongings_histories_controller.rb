class BelongingsHistoriesController < ApplicationController

  before_action :set_adventure

  def show
    @histories = @user.belongings_histories.where( book_id: @adventure.book_id )
  end

  def create
    @history = BelongingsHistory.find( params[:history_id])

    @belonging = Belonging.new( name: @history.name )
    @belonging.adventure = @adventure

    if @belonging.save
      redirect_to belongings_path
    else
      render :show
    end
  end

  def destroy
    @history = BelongingsHistory.find( params[:id])
    if @history.destroy
      redirect_to belongings_histories_path
    else
      render :show
    end
  end

end
