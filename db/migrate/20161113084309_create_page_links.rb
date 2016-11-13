class CreatePageLinks < ActiveRecord::Migration[5.0]
  def change
    create_table :page_links do |t|

      t.string :text
      t.references :src_page, index: true, null: false
      t.references :dst_page, null: false

      t.timestamps
    end
    add_foreign_key :page_links, :pages, column: :src_page_id
    add_foreign_key :page_links, :pages, column: :dst_page_id
  end
end
