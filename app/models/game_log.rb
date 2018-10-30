class GameLog < ApplicationRecord

  JOURNEY = 10
  FIGHT = 20
  ADVENTURE_UPDATE = 30
  ITEMS = 40

  belongs_to :adventure
  belongs_to :page

  serialize :log_data

  def self.gold_modification( adventure, amount, action )
    adventure.game_logs.create!( log_type: GameLog::ADVENTURE_UPDATE, adventure: adventure, page: adventure.current_page,
                                 log_data: { edit_typ: :gold, edit_action: action, amount: amount } )
  end

  def self.add_item( adventure, action, item )
    adventure.game_logs.create!( log_type: GameLog::ITEMS, adventure: adventure, page: adventure.current_page,
                                 log_data: { edit_typ: :item, edit_action: action, item_name: item } )
  end

end
