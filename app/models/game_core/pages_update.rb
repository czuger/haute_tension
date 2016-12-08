module GameCore::PagesUpdate

  def pages_content
    puts "Reading : #{url}"

    doc = Nokogiri::HTML( open( url ) )
    downloaded_page = doc.css('div.ob-text')

    downloaded_page.children.each do |node|
      if node.class == Nokogiri::XML::Element
        puts node.inspect
      end
    end
  end

  def update_page
    # puts "Updating : #{url}"

    doc = Nokogiri::HTML( open( url ) )
    downloaded_page = doc.css('div.ob-text')

    text = []
    monsters = []
    downloaded_page.children.each do |node|
      if node.class == Nokogiri::XML::Element
        node = check_for_link( node )
        # TODO : set up monsters
        # p node, monsters
        check_for_monsters( node, monsters )
        text << node.to_s.strip
      end

    end
    # p text, monsters
    update( text: text )
  end

  private

  def check_for_monsters( node, monsters )

    if node.children.first.name == 'strong'
      # p node
      # p node.text
      m = node.text.match( /(\d+)\ *VIE\ *:\ *(\d+)/ )
      if m
        p node
        m = Monster.find_or_create_by!( name: node.children.first.text, strength: m[1].to_i, hp: m[2].to_i )
        unless self.monsters.find( m.id )
          self.monsters << Monster.find_or_create_by!( name: node.children.first.text, strength: m[1].to_i, hp: m[2].to_i )
        end
      end
    end
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