FactoryGirl.define do
  factory :monster do
    name 'Basic monster'
    strength 10
    hp 10

    factory :weak_monster do
      name 'Weak monster'
      strength 8
      hp 8
    end

    factory :strong_monster do
      name 'Strong monster'
      strength 20
      hp 20
    end

  end
end
