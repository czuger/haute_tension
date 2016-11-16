class PageLink < ApplicationRecord

  belongs_to :src_page, class_name: 'Page'
  belongs_to :dst_page, class_name: 'Page'

end
