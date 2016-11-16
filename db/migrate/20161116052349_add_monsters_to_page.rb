class AddMonstersToPage < ActiveRecord::Migration[5.0]
  def change
    add_column :pages, :monsters, :string
  end
end
