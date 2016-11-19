class CreateJoinTablePageMonster < ActiveRecord::Migration[5.0]
  def change
    create_join_table :pages, :monsters do |t|
      t.index [:page_id, :monster_id], unique: true
      # t.index [:monster_id, :page_id]
    end
  end
end
