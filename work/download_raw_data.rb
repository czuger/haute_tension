require 'open-uri'
require 'fileutils'
require 'nokogiri'
require 'yaml' # Built in, no gem required

data_path = 'raw_data'

File.open( 'urls.txt', 'r' ).readlines.each do |url_line|

  next if url_line[0] == '#'

  path, start_url, _ = url_line.split( '|' )

  index = {}
  file_index = 0
  FileUtils.mkpath "#{data_path}/#{path}"

  urls = [ start_url ]

  until urls.empty?
    url_to_process = urls.shift

    unless index[ url_to_process ]

      puts "Downloading #{url_to_process}"

      download = open( url_to_process )
      file_path = "#{data_path}/#{path}/#{file_index}.html"
      IO.copy_stream(download, file_path)

      index[ url_to_process ] = file_path
      file_index += 1

      doc = Nokogiri::HTML( open( file_path ) )

      downloaded_page = doc.css('div.ob-text')

      downloaded_page.css( 'a' ).each do |paragraphe|
        # p paragraphe
        url = paragraphe.attributes['href'].value
        urls << url.gsub( 'www.lesitedontvousetesleheros.fr', 'lesitedontvousetesleheros.overblog.com' )
      end

    end
  end

  File.open( "#{data_path}/#{path}.yaml", 'w') {|f| f.write index.to_yaml } #Store

end