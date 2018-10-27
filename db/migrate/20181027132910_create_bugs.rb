class CreateBugs < ActiveRecord::Migration[5.0]
  def change
    create_table :bugs do |t|
      t.references :user, foreign_key: true, index: false, null: false
      t.references :page, foreign_key: true, index: false, null: false
      t.string :info, null: false
      t.boolean :fixed, null: false, default: false
      t.datetime :date_fixed

      t.timestamps
    end
  end
end
