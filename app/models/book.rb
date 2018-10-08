require 'nokogiri'
require 'pp'

class Book < ApplicationRecord
  has_many :pages

  TITLES = %w( pretre_jean_forteresse_alamuth pretre_jean_mines_roi_salomon pretre_jean_mysteres_bablylone pretre_jean_oeil_sphinx )

  def self.load_books
    ActiveRecord::Base.transaction do
      Book::TITLES.each do |title|
        b = Book.find_or_create_by!( name: title.humanize )

        db = YAML.load_file("work/raw_data/#{title}.yaml")

        db.values.each do |dl_data|
          page_data = YAML.load_file( "work/parsed_data/#{title}/#{dl_data[:page_index]}.html.yaml" )
          p = Page.where( page_hash: dl_data[:page_index] ).first_or_initialize
          p.text = page_data
          p.url = dl_data[:origin_url]
          p.book_id = b.id
          p.save!
        end
      end
    end
  end

end