class CreateInternalVariables < ActiveRecord::Migration[5.0]
  def change
    create_table :internal_variables do |t|
      t.string :var_name
      t.integer :var_int
      t.string :var_string

      t.timestamps
    end
    add_index :internal_variables, :var_name, unique: true
  end
end
