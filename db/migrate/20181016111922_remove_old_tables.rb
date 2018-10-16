class RemoveOldTables < ActiveRecord::Migration[5.0]
  def change
    drop_table :fight_monsters
    drop_table :monsters_pages
    drop_table :monsters_parsed_sections
    drop_table :monsters
    drop_table :page_links
    drop_table :parsed_section_links
    drop_table :parsed_sections
    drop_table :downloaded_sections
    drop_table :downloaded_books
  end
end
