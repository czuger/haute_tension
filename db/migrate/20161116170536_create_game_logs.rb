class CreateGameLogs < ActiveRecord::Migration[5.0]
  def change
    create_table :game_logs do |t|
      t.references :adventure, foreign_key: true, null: false, index: true
      t.references :page, foreign_key: true, null: false
      t.integer :log_type, null: false
      t.string :log_data

      t.timestamps
    end

  end
end