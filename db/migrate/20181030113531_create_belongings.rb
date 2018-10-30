class CreateBelongings < ActiveRecord::Migration[5.0]
  def change
    create_table :belongings do |t|
      t.references :adventure, foreign_key: true, null: false
      t.string :name, null: false

      t.timestamps
    end
  end
end
