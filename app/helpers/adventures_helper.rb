module AdventuresHelper

  def print_summary( adventure )
    adventure_text = adventure.current_page&.text&.first
    print_text_summary adventure_text
  end

  def print_text_summary( text, nb_words= 80 )
    if text
      return raw( text.gsub( '<p>', '' ).gsub( '</p>', '' ).truncate_words( nb_words ) )
    end

    ''
  end

end
