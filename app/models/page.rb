require 'nokogiri'
require 'open-uri'
require 'pp'
require 'json'

class Page < ApplicationRecord

  serialize :text
  has_many :page_links, foreign_key: :src_page_id
  has_many :pages, through: :page_links

  def self.download( book, link )
    puts "Downloading : #{link}"

    doc = Nokogiri::HTML(open( link ))
    downloaded_page = doc.css('div.ob-text')

    unless ( page = Page.find_by( url: link ) )
      text = []
      downloaded_page.css( 'p' ).each do |paragraphe|
        if paragraphe.children.first.name == 'text'
          text << paragraphe.children.first.to_s
        end
      end
      page = Page.create!( book_id: book.id, text: text, url: link )
    end

    downloaded_page.css( 'a' ).each do |paragraphe|
      # p paragraphe
      url = paragraphe.attributes['href'].value
      sub_page = Page.find_by( url: url ) # if the page already exist, we assume that it was downloaded
      sub_page = Page.download( book, url ) unless sub_page

      PageLink.find_or_create_by!( src_page_id: page.id, dst_page_id: sub_page.id ) do |pl|
        pl.text = paragraphe.children.first.to_s
      end
    end
    page
  end

end
