FactoryGirl.define do
  factory :fight_monster do
    adventure
    monster
    hp 10

    after( :create ) do |fm|
      # Link the created monster to the page monster of the adventure
      fm.adventure.page.monsters << fm.monster
    end
  end
end
