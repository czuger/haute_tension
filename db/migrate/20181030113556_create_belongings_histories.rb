class CreateBelongingsHistories < ActiveRecord::Migration[5.0]
  def change
    create_table :belongings_histories do |t|
      t.references :user, foreign_key: true, null: false
      t.references :book, foreign_key: true, index: false, null: false
      t.string :name, null: false

      t.timestamps
    end
  end
end
