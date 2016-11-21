class ChangeAdventureTrackers < ActiveRecord::Migration[5.0]
  def change
    add_column :adventures, :hp_max, :integer, null: false
    rename_column :adventures, :force, :strength
    add_column :adventures, :strength_max, :integer, null: false
    rename_column :adventures, :gourdes, :waterskins
    rename_column :adventures, :gourdes_remplies, :waterskins_max
    rename_column :adventures, :charisme, :charisma_avaliable
  end
end
