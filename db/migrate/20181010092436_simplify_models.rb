class SimplifyModels < ActiveRecord::Migration[5.0]
  def change
    add_column :books, :book_key, :string, null: false

    add_column :pages, :page_hash, :string, null: false
    add_index :pages, :page_hash, unique: true
    remove_index :pages, :url

    remove_foreign_key :adventures, :downloaded_books #, column: :downloaded_book_id
    rename_column :adventures, :downloaded_book_id, :book_id
    add_foreign_key :adventures, :books

    remove_foreign_key :adventures, :parsed_sections #, column: :current_parsed_section_id
    rename_column :adventures, :current_parsed_section_id, :current_page_id
    add_foreign_key :adventures, :pages, column: :current_page_id
  end
end
