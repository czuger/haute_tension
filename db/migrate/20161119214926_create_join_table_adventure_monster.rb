class CreateJoinTableAdventureMonster < ActiveRecord::Migration[5.0]
  def change
    create_table :fight_monsters do |t|
      t.references :adventure, foreign_key: true, null: false, index: true
      t.references :monster, foreign_key: true, null: false, index: true

      t.integer :hp, null: false
    end
  end
end
