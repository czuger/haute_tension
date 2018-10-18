FactoryBot.define do
  factory :game_log do

    page

    log_type {GameLog::JOURNEY}

    factory :game_log_fight do
      log_type {GameLog::FIGHT}
      log_data { ( {} ) }
    end

    factory :game_log_adventure_update do
      log_type {GameLog::ADVENTURE_UPDATE}
      log_data{ ( { edit_action: :down, edit_typ: :strength, amount: 3 } ) }
    end

  end
end
