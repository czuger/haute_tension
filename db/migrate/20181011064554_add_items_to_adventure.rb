class AddItemsToAdventure < ActiveRecord::Migration[5.0]
  def change
    add_column :adventures, :items, :string
    drop_table :items
    drop_table :adventures_items
  end
end
