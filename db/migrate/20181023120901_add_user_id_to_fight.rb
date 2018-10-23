class AddUserIdToFight < ActiveRecord::Migration[5.0]
  def change
    add_reference :fights, :user, foreign_key: true, null: false
    remove_index :fights, :book_id
  end
end
