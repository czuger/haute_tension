FactoryBot.define do

  factory :page do

    sequence :url do |n|
      "Page url #{n}"
    end

    sequence :page_hash do |n|
      "Page hash #{n}"
    end

    sequence :text do |n|
      "Lorem ipsum #{n}"
    end

    factory :fight_page_weak_monster do
      monsters { [ FactoryBot.create(:monster) ] }
    end

    factory :fight_page_two_weak_monsters do
      monsters { [ FactoryBot.create(:monster), FactoryBot.create(:weak_monster) ] }
    end

    factory :fight_page_hard_monster do
      monsters { [ FactoryBot.create(:strong_monster) ] }
    end

    factory :page_311_3 do
      url {'http://lesitedontvousetesleheros.overblog.com/311-3'}
    end

  end

end
