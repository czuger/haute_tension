class Book < ApplicationRecord
  belongs_to :first_page, optional: true, class_name: 'Page'
end
