class ParsedSectionLink < ApplicationRecord

  belongs_to :src_parsed_section, class_name: 'ParsedSection'
  belongs_to :dst_parsed_section, class_name: 'ParsedSection'

end
