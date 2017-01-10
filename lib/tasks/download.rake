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
      # Page.download_only( :forteresse_alamuth, 'http://www.lesitedontvousetesleheros.fr/1-61', downloaded )
      # downloaded.clear
      # # Page.download_only( :oeil_du_sphinx, 'http://www.lesitedontvousetesleheros.fr/2014/11/prologue-6.html', downloaded )
      # downloaded.clear
      # Page.download_only( :mines_roi_salomon, 'http://www.lesitedontvousetesleheros.fr/2014/12/prologue.html', downloaded )
      # downloaded.clear
      # Page.download_only( :mysteres_babylone, 'http://www.lesitedontvousetesleheros.fr/2014/12/prologue-2.html', downloaded )
      # downloaded.clear
      # URL parsing issue
      Page.download_only( :adorateurs_mal, 'http://www.lesitedontvousetesleheros.fr/la-saga-du-pr%C3%AAtre-jean', downloaded )
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
