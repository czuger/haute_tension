class AddItemsToAdventure2 < ActiveRecord::Migration[5.0]
  def change
    add_column :adventures, :items, :string, null: false
  end
end
