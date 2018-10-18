namespace :books do

    desc 'Build books database'
    task :build_db => :environment do

      TITLES = %w( pretre_jean_forteresse_alamuth pretre_jean_mines_roi_salomon pretre_jean_mysteres_bablylone pretre_jean_oeil_sphinx )
      BOOKS_NAMES = {
          pretre_jean_forteresse_alamuth: "1. La forteresse d'Alamuth", pretre_jean_oeil_sphinx: "2. L'oeil du Sphinx",
          pretre_jean_mines_roi_salomon: '3. Les mines du roi Salomon', pretre_jean_mysteres_bablylone: '4. Les mystères de Babylone'
      }

      ActiveRecord::Base.transaction do
        Book::TITLES.each do |title|
          b = Book.find_or_create_by!( book_key: title ) do |book|
            book.name = BOOKS_NAMES[title.to_sym]
          end

          db = YAML.load_file("work/raw_data/#{title}.yaml")

          db.values.each_with_index do |dl_data, index|
            page_data = YAML.load_file( "work/parsed_data/#{title}/#{dl_data[:page_index]}.html.yaml" )
            p = Page.where( page_hash: dl_data[:page_index] ).first_or_initialize
            p.text = page_data
            p.url = dl_data[:origin_url]
            p.book_id = b.id
            p.save!

            if index == 0
              b.first_page_id = p.id
              b.save!
            end
          end
        end
      end
  end
end
