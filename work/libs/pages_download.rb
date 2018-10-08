require 'fileutils'

module GameCore::PagesDownload

  def download_only( book, link, downloaded )

    return if downloaded.include?( link )

    puts "Downloading : #{link}"

    FileUtils.mkpath( "dl_pages/#{book.id}" )

    dl_page = book.downloaded_pages.where( url: link ).first_or_create

    doc = Nokogiri::HTML( open( link ) )
    File.open( "dl_pages/#{book.id}/#{dl_page.id}.html", 'w' ).write( doc )
    downloaded << link

    downloaded_page = doc.css('div.ob-text')

    downloaded_page.css( 'a' ).each do |par|
      url = par.attributes['href'].value
      Page.download_only( book, url, downloaded )
    end
  end

  # Page download
  def download( book, link )
    puts "Downloading : #{link}"

    FileUtils.mkpath( 'pages' )

    doc = Nokogiri::HTML( open( link ) )
    File.open( link, 'w' ).write( doc )

    downloaded_page = doc.css('div.ob-text')

    page = Page.find_by( url: link )
    page = Page.create!( book_id: book.id, url: link, text: [], monsters: [] ) unless page

    downloaded_page.css( 'a' ).each do |paragraphe|
      # p paragraphe
      url = paragraphe.attributes['href'].value
      sub_page = Page.find_by( url: url ) # if the page already exist, we assume that it was downloaded
      sub_page = Page.download( book, url ) unless sub_page

      PageLink.find_or_create_by!( src_page_id: page.id, dst_page_id: sub_page.id ) do |pl|
        pl.text = paragraphe.children.first.to_s
      end
    end

    page.update_page

    page
  end

end