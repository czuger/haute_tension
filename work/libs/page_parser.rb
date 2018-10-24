require 'nokogiri'
require 'pp'

class PageParser

  def update( directory, parsing_hash, pages_converter )

    page_path = parsing_hash[:file_path]
    @origin_url = parsing_hash[:origin_url]
    @pages_converter = pages_converter

    # pp @pages_converter

    doc = Nokogiri::HTML( open( page_path ) )
    read_page = doc.css('div.ob-text')

    text = []
    monsters = []
    read_page.children.each do |node|
      if node.class == Nokogiri::XML::Element
        node = process_links( node )

        # monsters = check_for_monsters( node, monsters )
        text << node.to_s.strip
      end
    end
    # puts 'Texte'
    # puts text
    # puts 'Monsters'
    # puts monsters
    # puts

    File.open( "parsed_data/#{directory}/#{File.basename(page_path)}.yaml", 'w' ) do |f|
      f.write( text.to_yaml )
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

  def process_links( node )
    node.xpath( './/a' ).each do |link|
      original_url = link.attributes['href'].value
      link.attributes['href'].value = "/adventures/read_choice/#{@pages_converter[original_url]}"
      link['class'] = 'pageLink'
    end
    node
  end

  # def check_for_link( node )
  #   if node.children.first.name == 'a'
  #     original_url = node.children.first.attributes['href'].value
  #     # page = Page.find_by_url( original_url )
  #     # raise "Unable to find page for #{original_url}" unless page
  #     # This read the rails route
  #     node.children.first.attributes['href'].value = "/adventures/read_choice/#{@pages_converter[original_url]}"
  #     node.children.first['class'] = 'pageLink'
  #     # p node.children.first
  #   elsif ( anchor = node.css( 'a' ) )
  #     if ! anchor.empty?
  #       original_url = anchor.first.attributes['href'].value
  #       # page = Page.find_by_url( original_url )
  #       # raise "Unable to page page for #{original_url}" unless page
  #       anchor.first.attributes['href'].value = "/adventures/read_choice/#{@pages_converter[original_url]}"
  #       node.children.first['class'] = 'pageLink'
  #     end
  #   end
  #   node
  # end

end