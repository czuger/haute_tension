module AdventuresHelper

  def print_summary( adventure )
    adventure_text = adventure.current_page&.text&.first
    if adventure_text
      return raw( adventure_text.gsub( '<p>', '' ).gsub( '</p>', '' ).truncate_words( 80 ) )
    end

    ''
  end

end
