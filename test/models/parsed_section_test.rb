require 'test_helper'

class ParsedSectionTest < ActiveSupport::TestCase

  test 'Create a parsed section' do
    parsed_section = create( :parsed_section )

    assert parsed_section
  end

end
