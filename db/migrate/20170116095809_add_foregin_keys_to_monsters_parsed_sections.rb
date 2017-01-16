class AddForeginKeysToMonstersParsedSections < ActiveRecord::Migration[5.0]
  def change
    add_foreign_key :monsters_parsed_sections, :monsters
    add_foreign_key :monsters_parsed_sections, :parsed_sections
  end
end
