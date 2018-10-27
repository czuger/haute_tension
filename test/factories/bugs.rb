FactoryBot.define do
  factory :bug do
    user { nil }
    page { nil }
    info { "MyString" }
    fixed { false }
    date_fixed { "2018-10-27 15:29:10" }
  end
end
