module GameCore::Fight

  include GameCore::Dices

  def fight( adventure, f_type )
    if f_type == 'seq'
      monsters.each do |monster|
        hero_attack_force = adventure.force + dices
        monster_attack_force = monster[ '' ]
      end
    else
    end
  end

end