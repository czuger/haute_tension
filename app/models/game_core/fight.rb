class Creature

  attr_reader :hp, :attack_force, :rolls

  def initialize( adventure, force, hp )
    @round = 0
    @adventure = adventure
    @force = force
    @hp = hp
  end

  def roll_attack
    roll = Hazard.s2d6
    @attack_force = @force + roll.result
    @rolls = roll.rolls
  end

end

class Hero < Creature
  def initialize( adventure )
    super( adventure, adventure.strength, adventure.hp )
  end
end

class LocalMonster < Creature
  attr_reader :name
  def initialize( adventure, fight, monster_index )
    # pp monster_data
    super( adventure, fight.send( "opponent_#{monster_index}_strength" ),
           fight.send( "opponent_#{monster_index}_life" ) )
    @name = fight.send( "opponent_#{monster_index}_name" )
  end
end

class GameCore::Fight

  def initialize( adventure, fight, monster_index )
    @adventure = adventure
    @hero = Hero.new( adventure )
    @monster = LocalMonster.new( adventure, fight, monster_index )
    @fight = fight
    round
  end

  private

  def round
    @hero.roll_attack
    @monster.roll_attack

    my_af = @hero.attack_force
    mn_af = @monster.attack_force
    hp_loss = my_af - mn_af

    if hp_loss > 0
      @fight.decrement!( "opponent_#{monster_index}_life", hp_loss )
    elsif hp_loss < 0
      @adventure.decrement!( :hp, -hp_loss )
    end

    @adventure.game_logs.create!(
      page_id: @adventure.current_page_id, log_type: GameLog::FIGHT, log_data: {
      hero_atk: my_af, monster_atk: mn_af, monster_hp_loss: hp_loss > 0 ? hp_loss : nil,
      hero_hp_loss: hp_loss < 0 ? -hp_loss : nil, hero_hp_remaining: @hero.hp, monster_hp_remaining: @monster.hp,
      monster_name: @monster.name, hero_rolls: @hero.rolls, monster_rolls: @monster.rolls }.compact
    )

  end
end