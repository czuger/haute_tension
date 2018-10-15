class CreateFights < ActiveRecord::Migration[5.0]
  def change
    create_table :fights do |t|

      t.string :opponent_1_name, null: false
      t.integer :opponent_1_strength, null: false, limit: 2
      t.integer :opponent_1_life, null: false, limit: 2
      t.integer :opponent_1_adv, limit: 2

      t.string :opponent_2_name
      t.integer :opponent_2_strength, limit: 2
      t.integer :opponent_2_life, limit: 2
      t.integer :opponent_2_adv, limit: 2

      t.string :opponent_3_name
      t.integer :opponent_3_strength, limit: 2
      t.integer :opponent_3_life, limit: 2
      t.integer :opponent_3_adv, limit: 2

      t.string :opponent_4_name
      t.integer :opponent_4_strength, limit: 2
      t.integer :opponent_4_life, limit: 2
      t.integer :opponent_4_adv, limit: 2

      t.string :opponent_5_name
      t.integer :opponent_5_strength, limit: 2
      t.integer :opponent_5_life, limit: 2
      t.integer :opponent_5_adv, limit: 2

      t.timestamps
    end
  end
end
