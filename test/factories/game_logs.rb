FactoryGirl.define do
  factory :game_log do

    adventure
    page

    log_type GameLog::JOURNEY

    factory :game_log_fight do
      log_type GameLog::FIGHT
      log_data( {} )
    end
  end
end
