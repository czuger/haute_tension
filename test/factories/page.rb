FactoryGirl.define do

  factory :page do

    book

    sequence :url do |n|
      "Dummy url #{n}"
    end

    text( [ 'dummy' ] )
    monsters 'dummy'

    factory :fight_page_weak_monster do
      monsters( [ { name: 'MONSTRE TOUT POURRI', force: 10, vie: 10 } ] )
    end

    factory :fight_page_two_weak_monsters do
      monsters( [ { name: 'MONSTRE TOUT POURRI 1', force: 10, vie: 10 }, { name: 'MONSTRE TOUT POURRI 2', force: 8, vie: 10 }  ] )
    end

    factory :fight_page_hard_monster do
      monsters( [ { name: 'MONSTRE DE LA MORT', force: 20, vie: 10 } ] )
    end

    factory :page_311_3 do
      url 'http://lesitedontvousetesleheros.overblog.com/311-3'
    end

  end

end
