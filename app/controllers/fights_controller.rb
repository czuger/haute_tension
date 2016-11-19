class FightsController < ApplicationController
  before_action :set_adventure, only: [ :show, :update, :add_monster, :remove_monster, :fight_monster ]

  def show
    if @adventure.fight_monsters.count == 0
      @adventure.page.monsters.each do |monster|
        @adventure.fight_monsters.create!( monster_id: monster.id, hp: monster.hp )
      end
    end
    @fight_monsters = @adventure.fight_monsters.includes( :monster )
  end

  def fight
    adventure = Adventure.find(params[:adventure_id])
    f = GameCore::Fight.new( adventure, params[ :f_type ] )
    result = nil
    ActiveRecord::Base.transaction do
      result = f.fight
    end
    if result == :hero_win
      redirect_to adventure_play_url( adventure_id: adventure )
    else
      redirect_to adventure_die_url( adventure_id: adventure )
    end
  end

  def add_monster
    @adventure.fight_monsters.create!( monster_id: @monster.id, hp: @monster.hp )
    redirect_to fight_url( @adventure )
  end

  def remove_monster
    @adventure.fight_monsters.delete( @fight_monster )
    redirect_to fight_url( @adventure )
  end

  def fight_monster
    GameCore::Fight.new( @adventure, @fight_monster )

    @adventure.fight_monsters.each do |fight_monster|
      fight_monster.destroy! if fight_monster.hp <= 0
    end

    if @adventure.fight_monsters.count > 0
      redirect_to fight_url( @adventure )
    else
      redirect_to adventure_play_url( @adventure )
    end

  end

  def update
    adventure_params = params.permit( :hp, :gold )
    
    %w( hp gold ).each do |field|
      adventure_params[field] = @adventure.hp + adventure_params[field].to_i if params['edit_action'] == 'up' && adventure_params[field]
      adventure_params[field] = @adventure.hp - adventure_params[field].to_i if params['edit_action'] == 'down' && adventure_params[field]
    end

    @adventure.game_logs.create!( page_id: @adventure.page_id, log_type: GameLog::ADVENTURE_UPDATE,
      log_data: { edit_action: params['edit_action'], edit_typ: params['edit_typ'] } )

    respond_to do |format|
      if @adventure.update( adventure_params )
        format.html { redirect_to adventure_play_url( @adventure ), notice: 'Aventure was successfully updated.' }
        format.json { render :show, status: :ok, location: adventure }
      else
        format.html { render :edit }
        format.json { render json: @adventure.errors, status: :unprocessable_entity }
      end
    end
  end

  private
    # Use callbacks to share common setup or constraints between actions.
    def set_adventure
      @adventure = Adventure.find( params[:id] ? params[:id] : params[:fight_id] )
      @monster = Monster.find( params[:monster_id] ) if params[:monster_id]
      @fight_monster = @adventure.fight_monsters.find( params[:fight_monster_id] ) if params[:fight_monster_id]
    end

    # Never trust parameters from the scary internet, only allow the white list through.
    def adventure_params
      params.require(:adventure).permit(:book_id, :adventure_id, :page_id, :hp, :gold, :gourdes, :gourdes_remplies, :rations, :charisme, :fight )
    end

end