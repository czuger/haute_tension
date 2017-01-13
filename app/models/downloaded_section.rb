class DownloadedSection < ApplicationRecord
  belongs_to :downloaded_book
  has_one :parsed_section
end
