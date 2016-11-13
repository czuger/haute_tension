class Aventure < ApplicationRecord
  belongs_to :book
  belongs_to :page
end
