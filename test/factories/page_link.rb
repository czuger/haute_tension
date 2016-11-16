FactoryGirl.define do

  factory :page_link do
    association :src_page, factory: :page
    association :dst_page, factory: :page
  end

end
