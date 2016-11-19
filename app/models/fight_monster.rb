class FightMonster < ApplicationRecord

  belongs_to :adventure
  belongs_to :monster

end
