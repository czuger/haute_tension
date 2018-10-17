class AddUserIdToAdventures < ActiveRecord::Migration[5.0]
  def change
    add_reference :adventures, :user, foreign_key: true, null: false
  end
end
