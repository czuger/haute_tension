class AddAdventureIdToUser < ActiveRecord::Migration[5.0]
  def change
    add_reference :adventures, :user, foreign_key: true, null: false
    add_column :users, :current_adventure_id, :integer
    add_foreign_key :users, :adventures, column: :current_adventure_id
  end
end
