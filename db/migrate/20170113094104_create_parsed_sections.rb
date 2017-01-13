class CreateParsedSections < ActiveRecord::Migration[5.0]
  def change
    create_table :parsed_sections do |t|
      t.references :downloaded_book, foreign_key: true, index: true, null: false
      t.references :downloaded_section, foreign_key: true, null: false
      t.string :content, null: false

      t.timestamps
    end
  end
end
