class AddFightReferencesToAdventure < ActiveRecord::Migration[5.0]
  def change
    add_column :adventures, :current_fight, :integer
    add_foreign_key :adventures, :fights, column: :current_fight
  end
end
