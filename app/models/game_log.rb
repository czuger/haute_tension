class GameLog < ApplicationRecord
  belongs_to :adventure
  belongs_to :page
end
