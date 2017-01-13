class ParsedSection < ApplicationRecord
  belongs_to :downloaded_book
  belongs_to :downloaded_section

  has_and_belongs_to_many :monsters

  serialize :content
end
