module GameCore::PagesUpdate

  def update_page
    puts "Updating : #{url}"

    doc = Nokogiri::HTML( open( url ) )
    downloaded_page = doc.css('div.ob-text')

    text = []
    monsters = []
    downloaded_page.children.each do |node|
      if node.class == Nokogiri::XML::Element
        if node.children.first.name == 'a'
          original_url = node.children.first.attributes['href'].value
          page = Page.find_by_url( original_url )
          raise "Unable to download page for #{original_url}" unless page
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