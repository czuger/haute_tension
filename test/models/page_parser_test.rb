require 'test_helper'

class PageParserTest < ActiveSupport::TestCase

  def test_1
    filename = 'pages/forteresse_alamuth/http___lesitedontvousetesleheros_overblog_com_107_30'
    result = GameCore::PageParser.new.parse_page( File.open( filename ).read )
    pp result
  end

end