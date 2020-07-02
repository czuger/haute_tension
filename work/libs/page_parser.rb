require 'nokogiri'
require 'pp'
require 'ostruct'

class PageParser

  def initialize
    @pages = []
  end

  def update( directory, parsing_hash, pages_converter )

    page_path = parsing_hash[:file_path]
    @origin_url = parsing_hash[:origin_url]
    @pages_converter = pages_converter

    # puts
    # puts "page_path = #{page_path}"
    # puts "@origin_url = #{@origin_url}"

    doc = Nokogiri::HTML( open( page_path ) )

    @pages << process_page( doc )

    # read_page = doc.css('div.ob-text')
    #
    # paragraphs = []
    # monsters = []
    # read_page.children.each do |node|
    #   if node.class == Nokogiri::XML::Element
    #     process_page( node )
    #
    #     # node = process_links( node )
    #
    #     # p node
    #
    #     # monsters = check_for_monsters( node, monsters )
    #     paragraphs << node.to_s.strip
    #
    #     # p paragraphs
    #
    #   end
    #
    #   # p paragraphs
    # end

    # File.open( "parsed_data/#{directory}/#{File.basename(page_path)}.yaml", 'w' ) do |f|
    #   f.write( paragraphs.to_yaml )
    # end
  end

  def save( directory )
    File.open( "parsed_data/#{directory}.yaml", 'w' ) do |f|
      f.write( @pages.to_yaml )
    end
  end

  private

  def check_for_monsters( page )
    monsters = []

    tmp_text = page[:text].dup

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
          monsters << { name: monster[1], force: monster[2].to_i, vie: monster[3].to_i, adjustment: adjustment }
        end
      end
    end

    monsters
  end

  def process_page( doc )
    page = { text: [], monsters: [] }

    doc.css( '.ob-section-text' ).each do |text_section|

      pictures = text_section.css( '.ob-img' )
      page[:text] << { type: :pics, sources: pictures.map{ |p| p['src'] } } unless pictures.empty?

      text_section.css( '.ob-text' ).xpath( './/p' ).each do |p|
        link = p.xpath( './/a' )

        unless link.empty?
          link = link[0]  # Shouldn't be more than one link
          page[:text] << { type: :link, text: link.text, src: link['href'] }
        else
          page[:text] << { type: :text, text: p.text }
        end
      end
    end

    page[:monsters] = check_for_monsters( page )

    page
  end

  # def process_links( node )
  #   node.xpath( './/a' ).each do |link|
  #     original_url = link.attributes['href'].value
  #
  #     # p original_url
  #
  #     link.attributes['href'].value = "/adventures/read_choice/#{@pages_converter[original_url]}"
  #     link['class'] = 'pageLink'
  #   end
  #   node
  # end

end