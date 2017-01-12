class CreateDownloadedPages < ActiveRecord::Migration[5.0]
  def change
    create_table :downloaded_pages do |t|

      t.string :url, null: false
      t.references :downloaded_book, index: true, foreign_key: true, null: false

      t.timestamps
    end

    add_index :downloaded_pages, :url, unique: true
  end
end
