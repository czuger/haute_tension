class GameLog < ApplicationRecord
  belongs_to :adventure
  belongs_to :src_page, class_name: 'Page', optional: true
  belongs_to :dst_page, class_name: 'Page', optional: true
  belongs_to :fight_page, class_name: 'Page', optional: true
end
