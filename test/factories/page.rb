FactoryGirl.define do

  factory :page do

    book

    sequence :url do |n|
      "Dummy url #{n}"
    end

    text( [ 'dummy' ] )

    factory :fight_page_weak_monster do
      monsters { [ FactoryGirl.create(:monster) ] }
    end

    factory :fight_page_two_weak_monsters do
      monsters { [ FactoryGirl.create(:monster), FactoryGirl.create(:weak_monster) ] }
    end

    factory :fight_page_hard_monster do
      monsters { [ FactoryGirl.create(:strong_monster) ] }
    end

    factory :page_311_3 do
      url 'http://lesitedontvousetesleheros.overblog.com/311-3'
    end

  end

end
