class AddBookToPage < ActiveRecord::Migration[5.0]
  def change
    add_reference :pages, :book, foreign_key: true, index: true, null: false
  end
end
