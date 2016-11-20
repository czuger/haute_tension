require 'test_helper'

class AventuresControllerTest < ActionDispatch::IntegrationTest
  setup do
    @adventure = create( :adventure )
  end

  test "should get index" do
    get adventures_url
    assert_response :success
  end

  test "should get new" do
    get new_adventure_url
    assert_response :success
  end

  test "should create adventure" do
    assert_difference('Adventure.count') do
      post adventures_url, params: {adventure: {book_id: @adventure.book_id, charisme: @adventure.charisme, force: @adventure.force, gold: @adventure.gold, gourdes: @adventure.gourdes, gourdes_remplies: @adventure.gourdes_remplies, hp: @adventure.hp, pages_id: @adventure.page_id, rations: @adventure.rations } }
    end

    assert_redirected_to adventure_url(Adventure.last)
  end

  test "should show adventure" do
    get adventure_url(@adventure)
    assert_response :success
  end

  test "should show play" do
    get adventure_play_url( @adventure )
    assert_response :success
  end

  test "should redirect after play choice" do
    get adventure_read_choice_url( adventure_id: @adventure.id, page_id: @adventure.page_id )
    assert_redirected_to adventure_play_url( @adventure )
  end

  test "should reroll" do
    get adventure_reroll_url( adventure_id: @adventure.id )
    assert_redirected_to @adventure
  end

  test "should roll dices" do
    get adventure_roll_dices_url( adventure_id: @adventure.id )
    assert_response :success
  end

  test "should show log" do
    log = create( :game_log_fight )
    get adventure_log_url( log.adventure )
    assert_response :success
  end

  test "should get edit" do
    get edit_adventure_url(@adventure)
    assert_response :success
  end

  test 'test gold, hp and rations increase decrease' do
    assert_difference '@adventure.reload.gold', -3 do
      patch adventure_url(@adventure), params: { gold: 3, edit_action: :down }
    end
    assert_difference '@adventure.reload.gold', 3 do
      patch adventure_url(@adventure), params: { gold: 3, edit_action: :up }
    end
    assert_difference '@adventure.reload.hp', -3 do
      patch adventure_url(@adventure), params: { hp: 3, edit_action: :down }
    end
    assert_difference '@adventure.reload.hp', 3 do
      patch adventure_url(@adventure), params: { hp: 3, edit_action: :up }
    end
    assert_difference '@adventure.reload.rations', -3 do
      patch adventure_url(@adventure), params: { rations: 3, edit_action: :down }
    end
    assert_difference '@adventure.reload.rations', 3 do
      patch adventure_url(@adventure), params: { rations: 3, edit_action: :up }
    end
    assert_redirected_to adventure_play_url(@adventure)
  end

  test "should update adventure" do
    patch adventure_url(@adventure), params: {adventure: {book_id: @adventure.book_id, charisme: @adventure.charisme, force: @adventure.force, gold: @adventure.gold, gourdes: @adventure.gourdes, gourdes_remplies: @adventure.gourdes_remplies, hp: @adventure.hp, pages_id: @adventure.page_id, rations: @adventure.rations } }
    assert_redirected_to adventure_play_url(@adventure)
  end

  test "should destroy adventure" do
    assert_difference('Adventure.count', -1) do
      delete adventure_url(@adventure)
    end

    assert_redirected_to adventures_url
  end
end
