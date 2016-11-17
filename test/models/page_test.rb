require 'test_helper'

class PageTest < ActiveSupport::TestCase

  test 'download page with embedded link' do
    p = create( :page )
    create( :page_311_3 )
    p.update_attribute( :url, 'test/static/47 - Le Site dont vous êtes le Héros.html' )
    p.update_page

    pp p
  end

end
