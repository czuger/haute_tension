desc 'Download data'
task :download => :environment do

  PageLink.delete_all
  Page.delete_all
  Book.delete_all

  b = Book.create!( name: "La forteresse d'alamuth" )
  p = Page.download( b, 'http://www.lesitedontvousetesleheros.fr/1-61' )
  b.update( first_page_id: p.id )

end
