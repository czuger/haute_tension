class GameLog < ApplicationRecord

  JOURNEY = 10
  FIGHT = 20
  ADVENTURE_UPDATE = 30

  belongs_to :adventure
  belongs_to :page

  serialize :log_data

end
