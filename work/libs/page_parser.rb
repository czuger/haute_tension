require 'nokogiri'

class PageParser

  def update( directory, parsing_hash )

    page_path = parsing_hash[:file_path]
    @origin_url = parsing_hash[:origin_url]

    doc = Nokogiri::HTML( open( page_path ) )
    downloaded_page = doc.css('div.ob-text')

    text = []
    monsters = []
    downloaded_page.children.each do |node|
      if node.class == Nokogiri::XML::Element
        # node = check_for_link( node )

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

  def check_for_monsters( node, monsters )

    if node.children.first.name == 'strong'
      # p node
      # p node.text
      m = node.text.match( /(\d+) *VIE *: *(\d+)/ )
      if m
        # p node
        monsters << { name: node.children.first.text, strength: m[1].to_i, hp: m[2].to_i }
      else
        puts "Monster in bad format : #{@origin_url}, #{node.inspect}"
      end
    end

    monsters
  end

  def check_for_link( node )
    if node.children.first.name == 'a'
      original_url = node.children.first.attributes['href'].value
      page = Page.find_by_url( original_url )
      raise "Unable to find page for #{original_url}" unless page
      # This read the rails route
      node.children.first.attributes['href'].value = Rails.application.routes.url_helpers.adventure_read_choice_path( 'CHANGE_ADVENTURE_ID', page.id )
      node.children.first['class'] = 'pageLink'
      # p node.children.first
    elsif ( anchor = node.css( 'a' ) )
      if ! anchor.empty?
        original_url = anchor.first.attributes['href'].value
        page = Page.find_by_url( original_url )
        raise "Unable to page page for #{original_url}" unless page
        anchor.first.attributes['href'].value = Rails.application.routes.url_helpers.adventure_read_choice_path( 'CHANGE_ADVENTURE_ID', page.id )
        node.children.first['class'] = 'pageLink'
      end
    end
    node
  end

end