namespace :data do
  namespace :download do

    desc 'Full download'
    task :full => :environment do
      PageLink.delete_all
      Page.delete_all
      Book.delete_all

      b = Book.create!( name: "La forteresse d'alamuth" )
      p = Page.download( b, 'http://www.lesitedontvousetesleheros.fr/1-61' )
      b.update( first_page_id: p.id )

    end

    desc 'Download pages only'
    task :pages => :environment do
      downloaded = Set.new

      books = [
        { name: "La forteresse d'alamuth", url: 'http://www.lesitedontvousetesleheros.fr/1-61' },
        { name: "L'oeil du sphinx", url: 'http://www.lesitedontvousetesleheros.fr/2014/11/prologue-6.html' },
        { name: 'Les mines du roi salomon', url: 'http://www.lesitedontvousetesleheros.fr/2014/12/prologue.html' },
        { name: 'Les mystères de babylone', url: 'http://www.lesitedontvousetesleheros.fr/2014/12/prologue-2.html' },
        { name: 'Les adorateurs du mal', url: 'http://www.lesitedontvousetesleheros.fr/la-saga-du-pr%C3%AAtre-jean' }
      ]

      books.each do |book_str|
        downloaded.clear
        dl_book = DownloadedBook.where( name: book_str[ :name ] ).first_or_create!( url: book_str[ :url ] )
        Page.download_only( dl_book, book_str[ :url ], downloaded )
      end
    end
  end

  desc 'Parse page'
  task :parse_page => :environment do
    Dir.glob( 'pages/forteresse_alamuth/*' ).each do |f|
      # next unless f == 'pages/forteresse_alamuth/http___lesitedontvousetesleheros_overblog_com_107_30'
      # next unless f == 'pages/forteresse_alamuth/http___lesitedontvousetesleheros_overblog_com_595_0'
      puts
      p f
      pp GameCore::PageParser.new.parse_page( File.open( f ).read )
    end
  end

  desc 'Update data'
  task :update => :environment do
    Page.all.each do |page|
      page.update_page
    end
  end

  desc 'Read datas'
  task :read => :environment do
    Page.all.each do |page|
      page.pages_content
    end
  end

end
