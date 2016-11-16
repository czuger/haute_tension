require 'nokogiri'
require 'open-uri'
require 'pp'
require 'json'

class Page < ApplicationRecord

  serialize :text
  serialize :monsters

  has_many :page_links, foreign_key: :src_page_id
  has_many :pages, through: :page_links
  belongs_to :book

  extend GameCore::PagesDownload
  include GameCore::PagesUpdate

end
