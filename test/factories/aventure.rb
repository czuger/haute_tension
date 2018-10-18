FactoryBot.define do

  factory :adventure do

    hp {20}
    hp_max {20}
    strength {12}
    strength_max {12}
    gold {20}
    rations {4}

    factory :fight_adventure_weak_monster do
      association :page, factory: :fight_page_weak_monster
    end

    factory :fight_adventure_hard_monster do
      association :page, factory: :fight_page_hard_monster
    end

    factory :fight_adventure_two_weak_monsters do
      association :page, factory: :fight_page_two_weak_monsters
    end

  end

end
