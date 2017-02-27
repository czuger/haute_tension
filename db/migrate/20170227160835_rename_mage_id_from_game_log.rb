class RenameMageIdFromGameLog < ActiveRecord::Migration[5.0]
  def change
    remove_foreign_key :game_logs, :pages
    rename_column :game_logs, :page_id, :parsed_section_id
    add_foreign_key :game_logs, :parsed_sections
  end
end
