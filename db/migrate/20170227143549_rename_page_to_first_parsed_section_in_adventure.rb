class RenamePageToFirstParsedSectionInAdventure < ActiveRecord::Migration[5.0]
  def change
    rename_column :adventures, :page_id, :current_parsed_section_id
    add_column :downloaded_books, :first_parsed_section_id, :integer
  end
end
