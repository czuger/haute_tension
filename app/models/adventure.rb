class Adventure < ApplicationRecord

  belongs_to :book
  belongs_to :current_page, class_name: 'Page'

  has_many :game_logs, dependent: :destroy
  has_many :fight_monsters, dependent: :destroy

  has_one :current_fight, class_name: 'Fight'

  serialize :items
end
