class CreateParsedSectionLinks < ActiveRecord::Migration[5.0]
  def change
    create_table :parsed_section_links do |t|
      t.integer :src_parsed_section
      t.integer :dst_parsed_section

      t.timestamps
    end

    remove_foreign_key :books, :pages
    remove_foreign_key :pages, :books
    remove_foreign_key :page_links, :pages

    drop_table :books
    drop_table :page_links
    drop_table :pages

  end
end

