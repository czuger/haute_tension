class DownloadedBook < ApplicationRecord
  has_many :downloaded_sections, dependent: :destroy
  has_many :adventures, dependent: :destroy

  belongs_to :first_parsed_section, class_name: 'ParsedSection', optional: true
end
