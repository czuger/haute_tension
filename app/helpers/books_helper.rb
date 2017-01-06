module BooksHelper

  def print_page_text( text )
    raw( text.gsub( 'CHANGE_ADVENTURE_ID', @adventure.id.to_s ) )
  end

end
