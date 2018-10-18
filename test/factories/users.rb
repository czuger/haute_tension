FactoryBot.define do
  factory :user do

    sequence :email do |n|
      "foo#{n}@bar.tender"
    end

    sequence :password do |n|
      "Password #{n}"
    end

  end
end
