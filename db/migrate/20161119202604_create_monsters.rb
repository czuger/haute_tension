class CreateMonsters < ActiveRecord::Migration[5.0]
  def change
    create_table :monsters do |t|
      t.string :name, null: false
      t.integer :strength, null: false
      t.integer :hp, null: false
      t.integer :adjustment

      t.timestamps
    end
    remove_column :pages, :monsters, :string
  end
end
