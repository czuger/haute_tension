class RenameBookFromAdventure < ActiveRecord::Migration[5.0]
  def change
    rename_column :adventures, :book_id, :downloaded_book_id

    remove_foreign_key :adventures, :pages
    add_foreign_key :adventures, :parsed_sections, column: :current_parsed_section_id

    remove_foreign_key :adventures, :books
    add_foreign_key :adventures, :downloaded_books, column: :downloaded_book_id
  end
end
