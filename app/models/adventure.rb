class Adventure < ApplicationRecord
  belongs_to :downloaded_book
  belongs_to :parsed_section, foreign_key: :current_parsed_section_id

  has_many :game_logs, dependent: :destroy
  has_many :fight_monsters, dependent: :destroy

  has_and_belongs_to_many :items
end
