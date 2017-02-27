FactoryGirl.define do
  factory :parsed_section do

    downloaded_book
    downloaded_section

    sequence :content do |n|
      "Parsed section #{n}"
    end
  end
end
