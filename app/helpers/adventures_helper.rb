module AdventuresHelper

  def log_fight_message( log )

    fight_result = log.hero_hp_loss ? :monster_win : :hero_win
    hp_loss = log.hero_hp_loss ? log.hero_hp_loss : log.monster_hp_loss
    hp_loss ||= 0

    ( '<' + log.monster_name + '>: ' + t( '.attack_sentence', yourforce: log.hero_atk, monsterforce: log.monster_atk ) +
      t( '.fight_result.' + fight_result.to_s, count: hp_loss ) )
  end

end
