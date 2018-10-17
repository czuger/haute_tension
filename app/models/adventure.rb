class Adventure < ApplicationRecord

  belongs_to :book
  belongs_to :current_page, class_name: 'Page'

  has_many :game_logs, dependent: :destroy

  belongs_to :current_fight, class_name: 'Fight', required: false

  serialize :items
end
