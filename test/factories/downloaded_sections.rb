FactoryGirl.define do
  factory :downloaded_section do
    sequence :url do |n|
      "Downloaded section #{n}"
    end
  end
end
