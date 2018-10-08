class CreateParsedSectionLinks < ActiveRecord::Migration[5.0]
  def change
    create_table :parsed_section_links do |t|
      t.integer :src_parsed_section
      t.integer :dst_parsed_section

      t.timestamps
    end

    add_column :pages, :page_hash, :string, null: false
    add_index :pages, :page_hash, unique: true
  end
end

