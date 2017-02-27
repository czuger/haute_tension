module GameCore
  class SectionParser

    def parse_page( downloaded_section )

      @dl_url = downloaded_section.url
      begin
        sub_parse_page( downloaded_section )
      rescue => e
        p [ downloaded_section.downloaded_book_id, downloaded_section.id, @dl_url ]
        p e
        pp e.backtrace
        exit
      end
    end

    def sub_parse_page( downloaded_section )

      f_path = "dl_pages/#{downloaded_section.downloaded_book_id}/#{downloaded_section.id}.html"
      page_content = File.open( f_path, 'r' ).read

      doc = Nokogiri::HTML( page_content )
      downloaded_page = doc.css('div.ob-text')

      @monsters = []
      @page_elements = []
      downloaded_page.children.each do |l1|
        next unless l1.class == Nokogiri::XML::Element

        parse_children( l1.children )
      end

      save_monster

      p_section = ParsedSection.find_or_initialize_by( downloaded_section_id: downloaded_section.id,
        downloaded_book_id: downloaded_section.downloaded_book_id ) do |section|
        section.content = @page_elements
      end

      p_section.save!

      db = DownloadedBook.find_by_url( downloaded_section.url )
      if db
        db.update( first_parsed_section_id: p_section.id )
      end

      p_section.monsters.clear

      unless GameCore::MonsterParser.exception_page?( @dl_url )
        @monsters.each do |monster|
          monster.create_monster_object( p_section )
        end
      else
        GameCore::MonsterParser.create_monster_object_on_exceptional_page( @dl_url, p_section )
      end
    end

    def parse_children( children )
      element_block = []
      children.each do |child|

        # p child

        next if child.to_s.strip.empty?

        result = nil
        case child.name
          when 'text'

            if !GameCore::MonsterParser.exception_page?( @dl_url ) && @monster
              @monster.process_stats( child )
            end

            result = { type: :text, text: child.to_s }
          when 'a'
            result = process_link( child )
          when 'strong'

            # Exception
            if GameCore::MonsterParser.exception_page?( @dl_url ) || child.children.first.to_s.strip =~ /^\d+$/
              # puts :exception
              result = { type: :text, text: child.children.first.to_s.strip }
            else
              save_monster
              @monster = GameCore::MonsterParser.new( child )

              # Some text has rubbish
              if @monster.rubbish?
                @monster = nil
                next
              end

              result = { type: :text, form: :strong, text: @monster.original_name }
            end
        end
        element_block << result if result
      end
      @page_elements << element_block unless element_block.empty?
    end

    def process_link( child )
      { type: :a, href: child[ 'href' ], text: child.children.first.to_s }
    end

    def save_monster
      if !GameCore::MonsterParser.exception_page?( @dl_url ) && @monster
        @monster.check!
        @monsters << @monster
      end
    end

  end
end