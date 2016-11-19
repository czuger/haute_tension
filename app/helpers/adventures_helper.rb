module AdventuresHelper

  def log_fight_message( log )

    log_data = log.log_data
    fight_result = log_data['hero_hp_loss'] ? :monster_win : :hero_win
    hp_loss = log_data['hero_hp_loss'] ? log_data['hero_hp_loss'] : log_data['monster_hp_loss']
    hp_loss ||= 0

    str = "<#{log_data['monster_name']}>: "
    str << t( '.attack_sentence', yourforce: log_data['hero_atk'], monsterforce: log_data['monster_atk'] )
    str << t( '.fight_result.' + fight_result.to_s, count: hp_loss )
    str
  end

  def adventure_edit_title( )
    t( ".#{@edit_action}_#{@edit_typ}_title" )
  end

end
