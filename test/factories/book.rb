FactoryBot.define do

  factory :book do

    sequence :name do |n|
      "Book #{n}"
    end

    sequence :book_key do |n|
      "Book key #{n}"
    end

    after(:create) do |book|
      page = create( :page, book: book )
      book.first_page = page
      book.save!
    end

  end

end
