class CreateBooks < ActiveRecord::Migration[5.0]
  def change
    create_table :books do |t|
      t.string :name, null: false
      t.references :first_page

      t.timestamps
    end
    add_foreign_key :books, :pages, column: :first_page_id
  end
end
