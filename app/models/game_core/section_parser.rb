module GameCore
  class SectionParser

    def parse_page( downloaded_sections )

      f_path = "dl_pages/#{downloaded_sections.downloaded_book_id}/#{downloaded_sections.id}.html"
      page_content = File.open( f_path, 'r' ).read

      doc = Nokogiri::HTML( page_content )
      downloaded_page = doc.css('div.ob-text')

      @monsters = []
      @page_elements = []
      downloaded_page.children.each do |l1|
        next unless l1.class == Nokogiri::XML::Element

        parse_children( l1.children )
      end

      p_section = ParsedSection.create!(
        downloaded_section_id: downloaded_sections.id, downloaded_book_id: downloaded_sections.downloaded_book_id,
        content: @page_elements
      )

      @monsters.each do |monster|
        p monster
        m = Monster.find_or_create_by!( name: monster[ :name ], hp: monster[ :vie ], strength: monster[ :force ],
                           adjustment: monster[ :adjustment ] )
        p_section.monsters << m
      end

      # { page_elements: @page_elements, monsters: @monsters }
    end

    def parse_children( children )
      element_block = []
      children.each do |child|
        result = nil
        case child.name
          when 'text'
            unless @monster_name
              result = { type: :text, text: child.to_s }
            else
              result = process_monster_stats( child )
            end
          when 'a'
            result = process_link( child )
          when 'strong'
            @monster_name = child.children.first.to_s.strip.upcase
            result = { type: :text, form: :strong, text: @monster_name }
        end
        element_block << result
      end
      @page_elements << element_block
    end

    def process_monster_stats( child )
      child = child.to_s.upcase
      r = child.match( /FORCE ?: ?([0-9]+)/ )
      force = r[1] if r

      r = child.match( /VIE ?: ?([0-9]+)/ )
      # Forteresse d'alamuth section 520 (http://www.lesitedontvousetesleheros.fr/2014/12/520.html)
      # Rats does not have life
      vie = r ? r[1] : 1

      adjustment = 0
      r = child.match( /AJUSTEMENT DEGATS ?: ?(\+|-) ?([0-9]+)/ )
      if r
        adjustment_sign = r[1] if r
        adjustment_value = r[2] if r
        adjustment = (adjustment_sign+adjustment_value).to_i
      end

      raise "No monster : #{child.inspect}" unless force && vie

      @monsters << { force: force, vie: vie, adjustment: adjustment, name: @monster_name }
      @monster_name = nil

      { type: :text, text: child }
    end

    def process_link( child )
      { type: :a, href: child[ 'href' ], text: child.children.first.to_s }
    end

  end
end