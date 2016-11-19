class Adventure < ApplicationRecord
  belongs_to :book
  belongs_to :page

  has_many :game_logs, dependent: :destroy
  has_and_belongs_to_many :monsters
end
