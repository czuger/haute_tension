FactoryGirl.define do
  factory :game_log do
    adventure nil
    src_page nil
    dst_page nil
    hero_atk 1
    monster_atk 1
    hero_hp_loss 1
    monster_hp_loss 1
    hero_hp_remaining 1
    monster_hp_remaining 1
    fight_round 1
    fight_page nil
  end
end
