FactoryGirl.define do
  factory :downloaded_book do

    sequence :name do |n|
      "Downloaded book #{n}"
    end

    sequence :url do |n|
      "Dummy url #{n}"
    end

    after( :create ) do |book|
      downloaded_section = create( :downloaded_section, downloaded_book: book )
      parsed_section = create( :parsed_section, downloaded_book: book, downloaded_section: downloaded_section )
      book.first_parsed_section = parsed_section
      book.save!
    end

  end
end
