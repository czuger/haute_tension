class FightsController < ApplicationController
  before_action :set_adventure

  def index
    @monsters = Monster.all.order( :name )
  end

  def new
    @fight = Fight.new
  end

  def show
    if @adventure.fight_monsters.count == 0 || @adventure.page.monsters.count >= 0
      @adventure.page.monsters.each do |monster|
        @adventure.fight_monsters.create!( monster_id: monster.id, hp: monster.hp )
      end
    end
    @fight_monsters = @adventure.fight_monsters.includes( :monster )
    @logs = GameLog.where( adventure_id: @adventure.id, page_id: @adventure.page_id, log_type: GameLog::FIGHT ).order( 'id DESC' )
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

  private
    # Use callbacks to share common setup or constraints between actions.
    def set_adventure
      @adventure = Adventure.find( params[:adventure_id] )
      # @monster = Monster.find( params[:monster_id] ) if params[:monster_id]
      # @fight_monster = @adventure.fight_monsters.find( params[:fight_monster_id] ) if params[:fight_monster_id]
    end

end
