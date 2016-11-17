class CreateGameLogs < ActiveRecord::Migration[5.0]
  def change
    create_table :game_logs do |t|
      t.references :adventure, foreign_key: true, null: false, index: true
      t.references :page, foreign_key: true, null: false
      t.boolean :fight, null: false, default: false
      t.string :monster_name
      t.integer :hero_atk
      t.integer :monster_atk
      t.integer :hero_hp_loss
      t.integer :monster_hp_loss
      t.integer :hero_hp_remaining
      t.integer :monster_hp_remaining
      t.integer :fight_round

      t.timestamps
    end

  end
end
