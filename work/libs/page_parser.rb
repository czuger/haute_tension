require 'nokogiri'
require 'pp'
require 'ostruct'
require 'json'

class PageParser

  def initialize( pages_converter )
    @pages = {}
    @pages_converter = pages_converter
  end

  def update( parsing_hash )

    page_path = parsing_hash[:file_path]
    page_hash = File.basename( parsing_hash[:file_path], '.html' )
    # origin_url = parsing_hash[:origin_url]

    # p page_path, origin_url

    doc = Nokogiri::HTML( open( page_path ) )

    @pages[ page_hash ] = process_page( doc, page_hash )
  end

  # This method save data for better view and test (useful for having an overview of regexp matches)
  def save( directory )
    File.open( "parsed_data/#{directory}.yaml", 'w' ) do |f|
      f.write( @pages.to_yaml )
    end
  end

  # This method prepare the data for future VueJs requests
  def prepare( directory )
    path = '../vue_js/data/' + directory

    FileUtils.mkpath( path )
    @pages.each do |key, data|
      File.open( "#{path}/#{key}.json", 'w' ) do |f|
        f.puts( JSON.pretty_generate( data ) )
      end
    end
  end

  private

  def check_for_monsters( page )
    monsters = []

    tmp_text = page[:paragraphs].dup

    until tmp_text.empty?
      text = tmp_text.shift

      if text[:type] == :text
        monster = text[:text].match( /(.*)FORCE[^\d]*(\d+) VIE[^\d]*(\d+)/ )

        adjustment = text[:text].match( /.*AJUSTEMENT[^\d]*(\d)/ )
        unless adjustment
          text = tmp_text.shift

          if text
            adjustment = text[:text].match( /.*AJUSTEMENT[^\d]*(\d)/ )
            if adjustment
              adjustment = adjustment[1]
            else
              tmp_text.unshift( text )
            end
          end
        end

        if monster
          m = { name: monster[1], force: monster[2].to_i, vie: monster[3].to_i }
          m[ :adjustment ] = adjustment if adjustment
          monsters << m
        end
      end
    end

    monsters
  end

  def process_page( doc, page_hash )
    page = { page_hash: page_hash, paragraphs: [], monsters: [] }

    doc.css( '.ob-section-text' ).each do |text_section|

      pictures = text_section.css( '.ob-img' )
      page[:paragraphs] << { type: :pics, sources: pictures.map{ |p| p['src'] } } unless pictures.empty?

      text_section.css( '.ob-text' ).xpath( './/p' ).each do |p|
        link = p.xpath( './/a' )

        unless link.empty?
          link = link[0]  # Shouldn't be more than one link
          page[:paragraphs] << { type: :link, text: link.text, hash: @pages_converter[link['href']] }
        else
          page[:paragraphs] << { type: :text, text: p.text }
        end
      end
    end

    page[:monsters] = check_for_monsters( page )

    page
  end

end