require 'open-uri'
require 'fileutils'
require 'nokogiri'
require 'digest'
require 'yaml'

data_path = 'raw_data'

File.open( 'urls.txt', 'r' ).readlines.each do |url_line|

  next if url_line[0] == '#'

  path, start_url, _ = url_line.split( '|' )

  index = {}
  FileUtils.mkpath "#{data_path}/#{path}"

  urls = [ start_url ]

  until urls.empty?
    url_to_process = urls.shift

    unless index[ url_to_process ]

      puts "Downloading #{url_to_process}"

      file_index = Digest::SHA2.hexdigest URI::split(url_to_process )[5]
      file_path = "#{data_path}/#{path}/#{file_index}.html"

      # download = open( url_to_process )
      # IO.copy_stream(download, file_path)

      index[ url_to_process ] = { file_path: file_path, origin_url: url_to_process, page_index: file_index }

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