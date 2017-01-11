module GameCore
  class PageParser

    def parse_page( page_content )

      doc = Nokogiri::HTML( page_content )
      downloaded_page = doc.css('div.ob-text')

      @monsters = []
      @page_elements = []
      downloaded_page.children.each do |l1|
        next unless l1.class == Nokogiri::XML::Element

        parse_children( l1.children )
      end
      { page_elements: @page_elements, monsters: @monsters }
    end

    def parse_children( children )
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
            @monster_name = child.children.first.to_s.chomp
            result = { type: :text, form: :strong, text: @monster_name }
        end
        @page_elements << result
      end
    end

    def process_monster_stats( child )
      child = child.to_s
      r = child.match( /FORCE ?: ?([0-9]+)/ )
      force = r[1] if r

      r = child.match( /VIE ?: ?([0-9]+)/ )
      vie = r[1] if r

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