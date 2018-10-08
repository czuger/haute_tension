require 'nokogiri'
require 'pp'

class Book < ApplicationRecord
  has_many :pages

  TITLES = %w( pretre_jean_forteresse_alamuth pretre_jean_mines_roi_salomon pretre_jean_mysteres_bablylone pretre_jean_oeil_sphinx )

  def self.load_books
    TITLES.each do |title|
      b = Book.find_or_create_by!( title: title.humanize )

      db = YAML.load_file("raw_data/#{title}.yaml")

      db.each do |dl_data|
        page_data = YAML.load_file( "parsed_data/#{dl_data[:page_index].yaml}" )

      end
    end
  end

end