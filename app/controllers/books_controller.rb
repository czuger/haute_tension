class BooksController < ApplicationController

  def show
    @adventure = Adventure.find(params[:adventure_id])
    @page = Page.find(params[:id])
    # to_i.to_s avoid injections
    @page.text.each{ |text| text.gsub!( 'CHANGE_ADVENTURE_ID', @adventure.id.to_s ) }
  end

end
