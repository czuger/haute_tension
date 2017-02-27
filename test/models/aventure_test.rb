require 'test_helper'

class AventureTest < ActiveSupport::TestCase

  test 'Create an adventure' do
    adv = create( :adventure )
    assert adv
  end

end
