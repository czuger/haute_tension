require 'test_helper'

class AdventureTrackersControllerTest < ActionDispatch::IntegrationTest

  setup do
    @adventure = create( :adventure )
  end

  test 'should update adventure tracker' do
    patch adventure_tracker_url(@adventure), params: {adventure: {book_id: @adventure.book_id, charisme: @adventure.charisma_avaliable, force: @adventure.strength, gold: @adventure.gold, gourdes: @adventure.waterskins, gourdes_remplies: @adventure.waterskins_max, hp: @adventure.hp, pages_id: @adventure.page_id, rations: @adventure.rations } }
    assert_redirected_to adventure_play_url(@adventure)
  end

  test 'test charisma' do
    patch adventure_tracker_url(@adventure), params: { charisma_avaliable: false }
    assert_redirected_to adventure_play_url(@adventure)
    refute @adventure.reload.charisma_avaliable
  end

  test 'decrease max should decrease corresponding value' do
    assert_difference "@adventure.reload.hp", -3 do
      patch adventure_tracker_url(@adventure), params: { hp_max: 3, edit_action: :down }
    end
    assert_redirected_to adventure_play_url(@adventure)
  end

  test 'increase max should not increase corresponding value' do
    assert_no_difference "@adventure.reload.hp" do
      patch adventure_tracker_url(@adventure), params: { hp_max: 3, edit_action: :up }
    end
    assert_redirected_to adventure_play_url(@adventure)
  end

  test 'test all values increase decrease' do
    %w( hp hp_max strength strength_max gold rations waterskins waterskins_max ).each do |p|

      assert_difference "@adventure.reload.#{p}", -3 do
        patch adventure_tracker_url(@adventure), params: { p => 3, edit_action: :down }
      end
      assert_redirected_to adventure_play_url(@adventure)

      assert_difference "@adventure.reload.#{p}", 3 do
        patch adventure_tracker_url(@adventure), params: { p => 3, edit_action: :up }
      end
      assert_redirected_to adventure_play_url(@adventure)
    end
  end

  test "should get edit" do
    get edit_adventure_tracker_url(@adventure)
    assert_response :success
  end
  
end
