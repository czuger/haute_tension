require_relative 'libs/page_parser'

require 'fileutils'
require 'yaml'

directories = %w( pretre_jean_forteresse_alamuth ) # pretre_jean_mines_roi_salomon pretre_jean_mysteres_bablylone pretre_jean_oeil_sphinx )

directories.each do |directory|
  FileUtils.mkpath "parsed_data/#{directory}"

  db = YAML.load_file("raw_data/#{directory}.yaml")
  pages_converter = Hash[ db.values.map{ |e| [ e[:origin_url], e[:page_index] ] } ]

  # p pages_converter

  p_pars = PageParser.new

  db.values.each do |parsing_hash|
    # p parsing_hash
    p_pars.update( directory, parsing_hash, pages_converter )
  end

  p_pars.save( directory )
end