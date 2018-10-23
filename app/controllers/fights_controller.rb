class FightsController < ApplicationController
  before_action :set_adventure

  def old_fights
    @fights = @user.fights.where( book_id: @adventure.book_id )
  end

  def old_fights_selection
    old_fight = Fight.find( params[:fight_id] )
    old_fight.dup.save
    @adventure.current_fight = old_fight

    if @adventure.save
      redirect_to fights_path, notice: 'Combat dupliqué'
    else
      @fights = @adventure.book.fights
      render :old_fights
    end
  end

  def new
    redirect_to fights_path if @adventure.current_fight
    @fight = Fight.new
    @title= 'Créer un combat'
  end

  def create
    @fight = Fight.new( fights_params )
    @fight.book_id = @adventure.book_id
    @fight.user_id = @user.id

    respond_to do |format|
      if save_fight
        format.html { redirect_to fights_path, notice: 'Combat crée' }
      else
        format.html { render :new }
      end
    end
  end

  def show
    @fight= @adventure.current_fight
    @read_only= true
    @title= 'Détail du combat'
  end

  def fight_monster
    @fight_result = GameCore::Fight.new( @adventure, params[:monster_index].to_i )
    @hero = @fight_result.hero
    @monster = @fight_result.monster
  end

  def destroy
    if @adventure.current_fight

      current_fight = @adventure.current_fight
      @adventure.current_fight_id = nil

      if @adventure.save && current_fight.destroy
        redirect_to new_fights_path, notice: 'Combat terminé'
      else
        render :show
      end
    end
  end

  private
    # Use callbacks to share common setup or constraints between actions.

    def fights_params
      params.require(:fight).permit('opponent_1_name', 'opponent_1_strength', 'opponent_1_life', 'opponent_1_adv', 'opponent_2_name', 'opponent_2_strength', 'opponent_2_life', 'opponent_2_adv', 'opponent_3_name', 'opponent_3_strength', 'opponent_3_life', 'opponent_3_adv', 'opponent_4_name', 'opponent_4_strength', 'opponent_4_life', 'opponent_4_adv', 'opponent_5_name', 'opponent_5_strength', 'opponent_5_life', 'opponent_5_adv')
    end

    def save_fight
      f_save = @fight.save
      @adventure.current_fight_id = @fight.id
      a_save = @adventure.save

      f_save && a_save && @fight.dup.save
    end

end
