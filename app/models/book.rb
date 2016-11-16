class Book < ApplicationRecord
  has_many :pages, dependent: :destroy
  belongs_to :first_page, optional: true, class_name: 'Page', dependent: :destroy

  has_many :adventures, dependent: :destroy
end
