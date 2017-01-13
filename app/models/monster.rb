class Monster < ApplicationRecord
  has_and_belongs_to_many :adventures

  has_and_belongs_to_many :parsed_section
end
