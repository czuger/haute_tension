require 'nokogiri'
require 'open-uri'
require 'pp'
require 'json'

class Page < ApplicationRecord

  serialize :text
  serialize :monsters

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

  def update_page
    puts "Updating : #{url}"

    doc = Nokogiri::HTML(open( url ))
    downloaded_page = doc.css('div.ob-text')

    text = []
    monsters = []
    downloaded_page.children.each do |node|
      if node.class == Nokogiri::XML::Element
        if node.children.first.name == 'a'
          original_url = node.children.first.attributes['href'].value
          page = Page.find_by_url( original_url )
          node.children.first.attributes['href'].value = Rails.application.routes.url_helpers.aventure_read_choice_path( 'CHANGE_ADVENTURE_ID', page.id )
        end
        if node.children.first.name == 'strong'
          p node
          p node.text
          m = node.text.match( /(\d+)\ *VIE\ *:\ *(\d+)/ )
          if m
            monsters << {
              name: node.children.first.text,
              force: m[1].to_i,
              vie: m[2].to_i
            }
          else
            monsters << {
              special: true
            }
          end
        end
        text << node.to_s.strip
      end

    end
    # p text, monsters
    update( text: text, monsters: monsters )
  end

end
