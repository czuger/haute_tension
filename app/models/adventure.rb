class Adventure < ApplicationRecord
  belongs_to :book
  belongs_to :page

  has_many :game_logs
end
