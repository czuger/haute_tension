class CreateDownloadedBooks < ActiveRecord::Migration[5.0]
  def change
    create_table :downloaded_books do |t|

      t.string :name, null: false
      t.string :url, null: false

      t.timestamps
    end

    add_index :downloaded_books, :name, unique: true
  end
end
