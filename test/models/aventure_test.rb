require 'test_helper'

class AventureTest < ActiveSupport::TestCase

  def test_dices
    Kernel.stubs( :rand ).returns( 6 )

    assert_equal 6, GameCore::Dices.d6

    assert_equal 12, GameCore::Dices.r2d6
    assert_equal 12, GameCore::Dices._2d6

    r = { result: 12, rolls: [ 6, 6 ] }
    assert_equal r, GameCore::Dices.s2d6
  end

end
