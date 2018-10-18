class RemoveInternalVariables < ActiveRecord::Migration[5.0]
  def change
    drop_table :internal_variables
  end
end
