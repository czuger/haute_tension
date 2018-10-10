class UpdateGameLog < ActiveRecord::Migration[5.0]
  def change
    remove_column :game_logs, :parsed_section_id, :integer
    add_reference :game_logs, :page, index: false, null: false, foreign_key: true
  end
end
