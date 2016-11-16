require 'test_helper'
require 'pp'

class FightTest < ActiveSupport::TestCase

  test 'Fight 1 weak monster' do
    adv = create( :fight_adventure_weak_monster )

    GameCore::Dices.stubs( :roll ).returns( 1 )

    fslot = GameCore::Fight.new( adv,'seq')
    assert_equal :hero_win, fslot.fight
  end

  test 'Fight 2 weak monsters' do
    adv = create( :fight_adventure_two_weak_monsters )

    GameCore::Dices.stubs( :roll ).returns( 1 )

    fslot = GameCore::Fight.new( adv,'seq')
    assert_equal :hero_win, fslot.fight

    GameLog.all.each do |game_log|
      p game_log
    end
  end

  test 'Fight 1 hard monster' do
    adv = create( :fight_adventure_hard_monster )

    GameCore::Dices.stubs( :roll ).returns( 1 )

    fslot = GameCore::Fight.new( adv,'seq')
    assert_equal :hero_die, fslot.fight
  end

  test 'Fight 1 weak monster - no stubs' do
    adv = create( :fight_adventure_weak_monster )

    # p adv.page

    fslot = GameCore::Fight.new( adv,'seq')
    fslot.fight

    GameLog.all.each do |log|
      p log
    end
  end


end
