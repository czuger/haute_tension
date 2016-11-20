FactoryGirl.define do

  factory :adventure do

    book
    page

    hp 20
    force 12
    gold 20
    rations 4

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
