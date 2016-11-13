class CreateAventures < ActiveRecord::Migration[5.0]
  def change
    create_table :aventures do |t|
      t.references :book, foreign_key: true, null: false
      t.references :page, foreign_key: true, null: false
      t.integer :hp, null: false
      t.integer :force, null: false
      t.integer :gold, null: false
      t.integer :gourdes, null: false, default: 2
      t.integer :gourdes_remplies, default: 2
      t.integer :rations, default: 4
      t.boolean :charisme, default: true

      t.timestamps
    end
  end
end
