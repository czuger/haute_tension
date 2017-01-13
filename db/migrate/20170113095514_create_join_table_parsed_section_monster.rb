class CreateJoinTableParsedSectionMonster < ActiveRecord::Migration[5.0]
  def change
    create_join_table :parsed_sections, :monsters do |t|
      # t.index [:parsed_sections_id, :monster_id], unique: true
      # t.index [:monster_id, :page_id]
    end

    add_index :monsters_parsed_sections, [:parsed_section_id, :monster_id], unique: true, name: :mps_ps_id_m_id
  end
end
