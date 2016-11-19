class CreateJoinTableAdventureMonster < ActiveRecord::Migration[5.0]
  def change
    create_join_table :adventures, :monsters do |t|
      t.index [:adventure_id, :monster_id]
      # t.index [:monster_id, :adventure_id]
    end
  end
end
