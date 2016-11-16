class CreateGameLogs < ActiveRecord::Migration[5.0]
  def change
    create_table :game_logs do |t|
      t.references :adventure, foreign_key: true, null: false, index: true
      t.references :src_page
      t.references :dst_page
      t.string :monster_name
      t.integer :hero_atk
      t.integer :monster_atk
      t.integer :hero_hp_loss
      t.integer :monster_hp_loss
      t.integer :hero_hp_remaining
      t.integer :monster_hp_remaining
      t.integer :fight_round
      t.references :fight_page

      t.timestamps
    end

    add_foreign_key :game_logs, :pages, column: :src_page_id
    add_foreign_key :game_logs, :pages, column: :dst_page_id
    add_foreign_key :game_logs, :pages, column: :fight_page_id

  end
end
