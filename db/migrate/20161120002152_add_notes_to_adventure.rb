class AddNotesToAdventure < ActiveRecord::Migration[5.0]
  def change
    add_column :adventures, :notes, :string
  end
end
