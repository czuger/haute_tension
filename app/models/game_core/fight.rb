require_relative 'dices'

class Creature
  attr_reader :hp
  def initialize( adventure, force, hp )
    @round = 0
    @adventure = adventure
    @force = force
    @hp = hp
  end
  def dead?
    @hp <= 0
  end
  def fight( opponent )
    @round += 1
    my_af = attack_force
    op_af = opponent.attack_force
    hp_loss = my_af - op_af
    if hp_loss > 0
      opponent.bleed( hp_loss )
    elsif hp_loss < 0
      bleed( -hp_loss )
    end
    @adventure.game_logs.create!(
      page_id: @adventure.page_id, type: GameLog::FIGHT, log_data: {
        hero_atk: my_af, monster_atk: op_af, monster_hp_loss: hp_loss > 0 ? hp_loss : nil,
        hero_hp_loss: hp_loss < 0 ? -hp_loss : nil, hero_hp_remaining: @hp, monster_hp_remaining: opponent.hp,
        fight_round: @round, monster_name: opponent.name }.compact
    )
  end
  def attack_force
    @force + GameCore::Dices.roll
  end
  def bleed( hp )
    @hp -= hp
  end
end

class Hero < Creature
  def initialize( adventure )
    super( adventure, adventure.force, adventure.hp )
  end
end

class Monster < Creature
  attr_reader :name
  def initialize( adventure, monster_data )
    # pp monster_data
    super( adventure, monster_data[ :force ], monster_data[ :vie ] )
    @name = monster_data[ :name ]
  end
end

class GameCore::Fight

  def initialize( adventure, f_type )
    @adventure = adventure
    @monsters = adventure.page.monsters
    @f_type = f_type
    @hero = Hero.new( adventure )
  end

  def fight
    if @f_type == 'seq'
      while( !@monsters.empty? && !@hero.dead? )
        monster = Monster.new( @adventure, @monsters.first )
        # p monster
        while( !monster.dead? && !@hero.dead? )
          @hero.fight( monster )
        end
        @monsters.shift if monster.dead?
      end
    elsif @f_type == 'togeth'
    else
      raise "Unknown fight type : #{@f_type}"
    end

    @adventure.update_attributes!( hp: @hero.hp )

    return :hero_die if @hero.dead?
    :hero_win
  end

end