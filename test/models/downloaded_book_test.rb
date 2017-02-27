require 'test_helper'

class DownloadedBookTest < ActiveSupport::TestCase

  test 'Create a downloaded book' do
    book = create( :downloaded_book )

    # pp book
    # pp book.first_parsed_section

    assert book
  end

end
