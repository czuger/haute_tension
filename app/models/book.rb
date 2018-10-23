require 'nokogiri'
require 'pp'

class Book < ApplicationRecord
  has_many :pages

  belongs_to :first_page, class_name: 'Page', required: false

  has_many :fights, dependent: :destroy
end