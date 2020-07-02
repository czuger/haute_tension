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

  # def check_for_monsters( node, monsters )
  #
  #   if node.children.first.name == 'strong'
  #     # p node
  #     # p node.text
  #     m = node.text.match( /(\d+) *VIE *: *(\d+)/ )
  #     if m
  #       # p node
  #       monsters << { name: node.children.first.text, strength: m[1].to_i, hp: m[2].to_i }
  #     else
  #       puts "Monster in bad format : #{@origin_url}, #{node.inspect}"
  #     end
  #   end
  #
  #   monsters
  # end

  def process_page( doc )
    page = []

    doc.css( '.ob-section-text' ).each do |text_section|

      pictures = text_section.css( '.ob-img' )
      page << OpenStruct.new( type: :pics, sources: pictures.map{ |p| p['src'] } ) unless pictures.empty?

      text_section.css( '.ob-text' ).xpath( './/p' ).each do |p|
        link = p.xpath( './/a' )

        unless link.empty?
          link = link[0]  # Shouldn't be more than one link
          page << OpenStruct.new( type: :link, text: link.text, src: link['href'] )
        else
          page << OpenStruct.new( type: :text, text: p.text )
        end
      end
    end

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