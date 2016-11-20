class CreateJoinTableAdventureItem < ActiveRecord::Migration[5.0]
  def change
    create_join_table :adventures, :items do |t|
      t.index [:adventure_id, :item_id], unique: true
      # t.index [:item_id, :adventure_id]
    end
  end
end
